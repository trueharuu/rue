import { type Classes, Client, type Types } from "@haelp/teto";
type Room = Classes.Room;
import { type Tracing, tracing } from "./tracing";
import { PermissionLevel } from "./parser/level";
import * as commands from "./parser/commands";
import { Context } from "./ctx";
import { Engine, FinesseType, type EngineOptions } from "./engine";
import { assert, Emoji, ty_assert } from "./util";

export interface Cfg {
    token: string;
    prefix: string;
    admins: Array<string>;
    dev_id: string | undefined;
}

export class Main {
    public connections: Map<string, Connection> = new Map();
    public main!: Client;
    public cluster!: Bun.Subprocess;
    public constructor(public readonly cfg: Cfg) {}

    public async spawn_cluster() {
        await this.ensure_exists();

        this.cluster = Bun.spawn(["./engine/target/release/engine_ipc"], {
            stdout: "inherit",
            stderr: "inherit",
        });
        console.log(this.cluster);
    }

    public async ensure_exists() {
        const build = Bun.spawn(["cargo", "build", "--release"], {
            cwd: "./engine",
            stdout: "inherit",
            stderr: "inherit",
        });
        await build.exited;
    }

    public async connect(): Promise<void> {
        this.main = await Client.create({
            token: this.cfg.token,
            ribbon: { transport: "json" },
        });

        tracing.thread("main").info("started");
        this.main.on("social.invite", async (c) => {
            if (!this.cfg.admins.includes(c.sender)) return;
            tracing
                .thread("main")
                .info(`invited to ${c.roomid} by ${c.sender}`);
            const con = await this.spawn(c.roomid);
            await con.connect();
        });

        if (this.cfg.dev_id !== undefined) {
            try {
                const con = await this.spawn(this.cfg.dev_id);
                await con.connect();
            } catch (e) {
                tracing
                    .thread("main")
                    .error(`failed to auto-join dev room: ${e}`);
                this.connections.delete(this.cfg.dev_id);
            }
        }
    }

    public async spawn(id: string): Promise<Connection> {
        let con = this.connections.get(id);
        if (!con) {
            con = new Connection(this, id);
            this.connections.set(id, con);
        }

        return con;
    }

    public async emit(
        id: string,
        f: (this: Connection) => unknown,
    ): Promise<unknown> {
        const con = await this.spawn(id);
        return con.recv(f);
    }
}

export class Connection {
    public client!: Client;
    public room!: Room;
    public tracing: Tracing;
    public engine?: Engine;
    public engine_options: EngineOptions = {
        pps: 3,
        vision: 7,
        finesse: FinesseType.Human,
    };
    public constructor(
        public src: Main,
        public id: string,
    ) {
        this.tracing = tracing.thread(this.id);
    }

    public game(): Classes.Game {
        ty_assert(this.client.game);
        return this.client.game!;
    }

    public async connect(): Promise<void> {
        if (this.client !== undefined) {
            return;
        }

        this.client = await Client.create({
            token: this.src.cfg.token,
            game: {
                handling: {
                    arr: 0,
                    cancel: false,
                    das: 1,
                    dcd: 0,
                    safelock: false,
                    may20g: true,
                    sdf: 41,
                    ihs: "tap",
                    irs: "tap",
                },
            },
        });
        this.room = await this.client.rooms.join(this.id);
        this.tracing.info("joined");

        await this.enable(false);

        this.engine = new Engine(this);
        await this.engine.init();

        assert(this.engine);
        await this.engine.request({ packet: "Ping", data: null });

        this.client.on("room.player.remove", (c) => {
            if (this.room.players.filter((x) => !x.bot).length === 0) {
                this.tracing.warn("no players left, leaving room");
                this.disconnect();
            }
        });

        this.client.on("client.game.round.start", async ([c]) => {
            assert(this.engine);
            if (!this.engine.socket) {
                await this.engine.init();
            }

            await this.engine.request({
                packet: "Setup",
            });
            c(async (t) => {
                t.engine.queue.minLength = 100;
                assert(this.engine);

                return await this.engine.tick(t);
            });
        });

        this.client.on("client.game.end", async () => {
            if (this.engine) {
                this.engine.kill();
            }
        });

        this.client.on("room.update.bracket", async (c) => {
            if (c.uid !== this.client.user.id) {
                return;
            }

            if (c.bracket === "player" && !this.enabled) {
                try {
                    await this.client.room?.switch("spectator");
                } catch (_e) {}
            } else if (c.bracket === "spectator" && this.enabled) {
                try {
                    await this.client.room?.switch("player");
                } catch (_e) {}
            }
        });

        this.client.on("room.update", async (_c) => {});

        this.client.on("room.chat", async (c) => {
            for (const name in commands) {
                const cmd = commands[name as keyof typeof commands];
                try {
                    const t = await cmd.parse(this, c.content);
                    if (t === undefined) {
                        continue;
                    }

                    if (c.user.role === "bot") {
                        continue;
                    }

                    try {
                        await cmd.exec(
                            new Context(
                                this,
                                c,
                                t.args as never,
                                t.flags as never,
                            ),
                        );
                    } catch (e) {
                        await this.room.chat(String(e));
                    }
                } catch (e) {
                    await this.room.chat(String(e));
                }
            }
        });
    }

    public permissionLevel(id: string): PermissionLevel {
        if (this.src.cfg.admins.includes(id)) {
            return PermissionLevel.Sysop;
        }

        if (this.room.owner === id || this.room.creator === id) {
            return PermissionLevel.Host;
        }

        const player = this.room.players.find((p) => p._id === id);
        if (player) {
            if (player.bracket === "player") {
                return PermissionLevel.Player;
            }

            return PermissionLevel.Spectator;
        }

        return PermissionLevel.None;
    }

    // true if source>target
    public permissionMatches(source: PermissionLevel, target: PermissionLevel) {
        if (target === PermissionLevel.None) {
            return true;
        }

        if (target === PermissionLevel.Spectator) {
            return (
                source === PermissionLevel.Spectator ||
                source === PermissionLevel.Player ||
                source === PermissionLevel.Host ||
                source === PermissionLevel.Sysop
            );
        }

        if (target === PermissionLevel.Player) {
            return (
                source === PermissionLevel.Player ||
                source === PermissionLevel.Host ||
                source === PermissionLevel.Sysop
            );
        }

        if (target === PermissionLevel.Host) {
            return (
                source === PermissionLevel.Host ||
                source === PermissionLevel.Sysop
            );
        }

        if (target === PermissionLevel.Sysop) {
            return source === PermissionLevel.Sysop;
        }

        return false;
    }

    public async disconnect(): Promise<void> {
        if (this.room) {
            await this.room.leave();
            await this.client.destroy();
            this.src.connections.delete(this.id);
            this.tracing.warn("left");
        }
    }

    public async recv(f: (this: Connection) => unknown): Promise<unknown> {
        return f.bind(this)();
    }

    private enabled: boolean = false;
    private pendingEnable: boolean = false;
    public async enable(report: boolean = true): Promise<void> {
        try {
            this.enabled = true;
            await this.room.switch("player");

            if (report) {
                await this.room.chat(`${Emoji.Checkmark}`);
            }
        } catch (_e) {
            if (report) {
                await this.room.chat(`${Emoji.X}`);
            }
        }
    }

    public async disable(report: boolean = true): Promise<void> {
        try {
            this.enabled = false;
            await this.room.switch("spectator");
            if (report) {
                await this.room.chat(`${Emoji.Checkmark}`);
            }
        } catch (_e) {
            if (report) {
                await this.room.chat(`${Emoji.X}`);
            }
        }
    }
}
