import { Emoji, table } from "../util";
import { FinesseType } from "../engine";
import { args, commands } from "./_";
import * as err from "./err";
import { type Argument, Command } from "./handler";
import { PermissionLevel } from "./level";

export const help = new Command(
    {
        name: "help",
        aliases: ["h", "commands"],
        description: "prints this message",
        permission: PermissionLevel.None,
        arguments: {
            command: args
                .optional(args.word())
                .describe("the command to get help for"),
        },
        flags: {},
    },

    async (ctx) => {
        if (ctx.args.command) {
            const cmd = Object.values(commands).find(
                (c) =>
                    c.options.name.toLowerCase() ===
                    ctx.args.command?.toLowerCase(),
            );

            if (cmd) {
                let argdesc = "";
                for (const arg in cmd.options.arguments) {
                    const parser = cmd.options.arguments[
                        arg as keyof typeof cmd.options.arguments
                    ] as Argument<unknown>;

                    if (parser.description) {
                        argdesc += `\n    ${arg}: ${parser.description}`;
                    }
                }

                for (const flag in cmd.options.flags) {
                    const parser = cmd.options.flags[
                        flag as keyof typeof cmd.options.flags
                    ] as Argument<unknown>;

                    if (parser.description) {
                        argdesc += `\n    --${flag}: ${parser.description}`;
                    }
                }
                await ctx.reply(
                    `${ctx.client.src.cfg.prefix}${await cmd.usage()}\n    ${cmd.options.description}${cmd.options.aliases.length > 0 ? `\n    aliases: ${cmd.options.aliases.join(", ")}` : ""}${argdesc ? `\n\narguments:${argdesc}` : ""}`,
                );
            } else {
                await ctx.reply(`command '${ctx.args.command}' not found`);
            }
        } else {
            await ctx.reply(
                `prefix is ${ctx.client.src.cfg.prefix}\n\ncommands: ${Object.values(
                    commands,
                )
                    .map((c) => `${c.options.name}`)
                    .join(", ")}`,
            );
        }
    },
);

export const die = new Command(
    {
        name: "die",
        aliases: ["kill"],
        description: "kills the bot",
        permission: PermissionLevel.Host,
        arguments: {},
        flags: {},
    },
    async (ctx) => {
        await ctx.reply(":oyes:/");
        await ctx.client.disconnect();
    },
);

export const pps = new Command(
    {
        name: "pps",
        aliases: [],
        description: "sets pps",
        permission: PermissionLevel.Spectator,
        arguments: {
            pps: args
                .optional(args.float(0.5, 20))
                .describe("pieces per second"),
        },
        flags: {},
    },
    async (ctx) => {
        const prev = ctx.client.engine_options.pps;
        if (ctx.args.pps === null) {
            await ctx.reply(`${prev}`);
            return;
        }

        const l = ctx.client.permissionLevel(ctx.message.user._id!);
        if (!ctx.client.permissionMatches(l, PermissionLevel.Host)) {
            throw err.missing_permissions(PermissionLevel.Host, l);
        }

        if (ctx.args.pps >= 7) {
            ctx.client.engine_options.finesse = FinesseType.Instant;
            await ctx.reply(
                `${Emoji.Warning} \`instant\` finesse automatically set for 7+ pps`,
            );
        }

        ctx.client.engine_options.pps = ctx.args.pps;
        await ctx.reply(`${Emoji.Checkmark}`);
    },
);

export const finesse = new Command(
    {
        name: "finesse",
        aliases: [],
        description: "sets finesse type",
        permission: PermissionLevel.Host,
        arguments: {
            finesse: args
                .optional(args.enumerable(FinesseType))
                .describe("finesse type"),
        },
        flags: {},
    },
    async (ctx) => {
        const prev = ctx.client.engine_options.finesse;
        if (ctx.args.finesse === null) {
            await ctx.reply(`${prev}`);
            return;
        }

        ctx.client.engine_options.finesse = ctx.args.finesse;
        await ctx.reply(`${Emoji.Checkmark}`);
    },
);

export const vision = new Command(
    {
        name: "vision",
        aliases: ["see"],
        description: "sets vision",
        permission: PermissionLevel.Spectator,
        arguments: {
            vision: args.optional(args.int(0, 100)).describe("vision"),
        },
        flags: {},
    },
    async (ctx) => {
        const prev = ctx.client.engine_options.vision;
        if (ctx.args.vision === null) {
            await ctx.reply(`${prev}`);
            return;
        }

        const l = ctx.client.permissionLevel(ctx.message.user._id!);
        if (!ctx.client.permissionMatches(l, PermissionLevel.Host)) {
            throw err.missing_permissions(PermissionLevel.Host, l);
        }

        ctx.client.engine_options.vision = ctx.args.vision;
        await ctx.reply(`${Emoji.Checkmark}`);
    },
);

export const enable = new Command(
    {
        name: "enable",
        aliases: ["e"],
        description: "enables gameplay",
        permission: PermissionLevel.Host,
        arguments: {},
        flags: {},
    },
    async (ctx) => {
        await ctx.client.enable();
    },
);

export const disable = new Command(
    {
        name: "disable",
        aliases: ["d"],
        description: "disables gameplay",
        permission: PermissionLevel.Host,
        arguments: {},
        flags: {},
    },
    async (ctx) => {
        if (ctx.client.engine?.socket) {
            await ctx.reply(`cannot disable ingame`);
            return;
        }
        await ctx.client.disable();
    },
);

export const settings = new Command(
    {
        name: "settings",
        aliases: [],
        description: "shows current engine settings",
        permission: PermissionLevel.Spectator,
        arguments: {},
        flags: {},
    },
    async (ctx) => {
        const opts = ctx.client.engine_options;
        await ctx.reply(table(opts));
    },
);

export const where = new Command(
    {
        name: "where",
        aliases: [],
        description: "shows current location",
        permission: PermissionLevel.Sysop,
        arguments: {},
        flags: {},
    },
    async (ctx) => {
        for (const [key, value] of ctx.client.src.connections) {
            let settings = value.engine?.socket
                ? `${value.engine_options.pps}pps, see${value.engine_options.vision}`
                : "disabled";
            await ctx.reply(
                `${value.room.id}: ${value.room.state} (${settings})`,
            );
        }
    },
);

export const who = new Command(
    {
        name: "who",
        aliases: [],
        description: "user info",
        permission: PermissionLevel.None,
        arguments: {
            user: args.word(),
        },
        flags: {},
    },
    async (ctx) => {
        try {
            const user = await ctx.client.src.main.api.users.get({
                username: ctx.args.user,
            });
            await ctx.reply(
                `${user.username}\n| id: ${user._id}\n| role: ${ctx.client.permissionLevel(user._id)}`,
            );
        } catch (e) {
            await ctx.reply(`user '${ctx.args.user}' not found`);
            return;
        }
    },
);
