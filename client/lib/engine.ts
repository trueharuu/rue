import type { Types } from "@haelp/teto";
import type { Connection } from "./connection";
import type { Engine as CEngine } from "@haelp/teto";
import { ty_assert } from "./util";
import type { adapters } from "@haelp/teto/utils";
import type { Game } from "@haelp/teto/types";
/// engine connection process
/// messages are newline-suffixed JSON
/// every message is of the format `{packet: string, data: unknown}`

export interface Packet<Hint = unknown> {
    packet: string;
    data?: Hint;
}

export enum FinesseType {
    Human = "human",
    Instant = "instant",
}
export interface EngineOptions {
    pps: number; // pieces per second
    vision: number; // how many pieces in the queue to see
    finesse: FinesseType;
}

export class Engine {
    public socket?: Bun.Socket;
    private pending?: (value: Packet) => void;
    private buffer: string = "";

    public constructor(public connection: Connection) {}

    public async kill() {
        if (this.socket) {
            this.socket.end();
            this.socket = undefined;
            this.pending = undefined;
            this.buffer = "";
        }
    }

    public async init() {
        const self = this;
        this.socket = await Bun.connect({
            hostname: "localhost",
            port: 9000,
            socket: {
                data(_socket, data) {
                    self.buffer += data.toString();

                    let newlineIndex: number;
                    newlineIndex = self.buffer.indexOf("\n");
                    while (newlineIndex !== -1) {
                        const line = self.buffer.slice(0, newlineIndex);
                        self.buffer = self.buffer.slice(newlineIndex + 1);

                        if (line.trim().length === 0) {
                            newlineIndex = self.buffer.indexOf("\n");
                            continue;
                        }

                        if (self.pending) {
                            const response: Packet = JSON.parse(line);
                            self.connection.tracing.debug(`recieve ${line}`);
                            self.pending(response);
                            self.pending = undefined;
                        } else {
                            self.connection.tracing.warn(
                                `received response without pending request: ${line}`,
                            );
                        }

                        newlineIndex = self.buffer.indexOf("\n");
                    }
                },
                error(_socket, error) {
                    self.connection.tracing.error(`socket error: ${error}`);
                },
                close() {
                    self.connection.tracing.error("socket closed");
                },
            },
        });
    }

    public request(data: Packet): Promise<Packet> {
        if (!this.socket) {
            throw new Error("engine not connected");
        }

        if (this.pending) {
            throw new Error("request already in progress");
        }

        const message = `${JSON.stringify(data)}\n`;
        this.connection.tracing.debug(`send ${message}`);
        this.socket.write(message);

        return new Promise((resolve, _reject) => {
            this.pending = resolve;
        });
    }

    public readonly fps: number = 60;
    private acc: number = 0;
    public async tick(t: Types.Game.Tick.In): Promise<Types.Game.Tick.Out> {
        this.acc += this.connection.engine_options.pps / this.fps;

        const keys = [];

        while (this.acc >= 1) {
            const ks = await this.keys(t.engine);

            keys.push(...this.frames(t.engine, ks));

            this.acc -= 1;
        }

        return { keys };
    }

    public async keys(engine: CEngine.Engine): Promise<Array<Types.Game.Key>> {
        const hold = engine.held?.toLowerCase() ?? null;
        const active = engine.falling.symbol.toLowerCase();
        const next_queue = engine.queue
            .slice(0, Math.max(this.connection.engine_options.vision - 2, 0))
            .map((p) => p.toLowerCase());
        const combo = engine.stats.combo + 1;
        const board = this.serialize_board(engine);

        const response = (await this.request({
            packet: "Path",
            data: {
                hold,
                queue: [active, ...next_queue],
                combo,
                board,
                b2b: engine.stats.b2b,
                incoming_garbage: engine.garbageQueue.queue.map((x) => x.size),
            },
        })) as Packet;

        const { piece, finesse } = response.data as {
            piece: string;
            finesse: Array<Types.Game.Key>;
        };

        if (piece === undefined) {
            return ["hardDrop"];
        }

        const is_held = active.toLowerCase() !== piece.toLowerCase();
        if (is_held) {
            finesse.unshift("hold");
        }
        if (finesse.length === 1) {
            finesse.unshift("softDrop");
        }

        return finesse;
    }

    // @trueharuu todo
    public serialize_board(engine: CEngine.Engine): string {
        return engine.board.state
            .map((x) => x.map((y) => (y === null ? "_" : "X")).join(""))
            .join("|");
    }

    public frames(
        c: CEngine.Engine,
        ks: Array<Types.Game.Key>,
    ): Array<Types.Game.Tick.Keypress> {
        const keys: Array<Types.Game.Tick.Keypress> = [];
        // keys.push({
        //   frame: c.frame,
        //   data: { key: "softDrop", subframe: 0.0 },
        //   type: "keydown",
        // });

        if (this.connection.engine_options.finesse === FinesseType.Human) {
            const delta =
                this.fps / this.connection.engine_options.pps / ks.length;
            for (let i = 0; i < ks.length; i++) {
                const z = ks[i];
                ty_assert(z);
                const whole = c.frame + Math.floor(i * delta);
                const fract = i * delta - Math.floor(i * delta);

                keys.push({
                    frame: whole,
                    data: { key: z, subframe: fract },
                    type: "keydown",
                });
                keys.push({
                    frame: whole,
                    data: { key: z, subframe: fract + 0.1 },
                    type: "keyup",
                });
            }
        } else if (
            this.connection.engine_options.finesse === FinesseType.Instant
        ) {
            let r_subframe = 0;
            for (const key of ks) {
                keys.push({
                    frame: c.frame,
                    type: "keydown",
                    data: {
                        key,
                        subframe: r_subframe,
                    },
                });

                if (key === "softDrop") {
                    r_subframe += 0.1;
                }

                keys.push({
                    frame: c.frame,
                    type: "keyup",
                    data: {
                        key,
                        subframe: r_subframe,
                    },
                });
            }
        }

        return keys;
    }
}
