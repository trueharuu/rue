import type { Connection } from "../connection";
import type { Context } from "../ctx";
import * as err from "./err";
import type { PermissionLevel } from "./level";

export interface CommandOptions<T extends Arguments, F extends Arguments> {
    name: string;
    description: string;
    aliases: string[];
    arguments: T;
    flags: F;
    permission: PermissionLevel;
}

export type Exec<T extends Arguments, F extends Arguments> = (
    ctx: Context<T, F>,
) => unknown;

export type Arguments = Record<string, Argument<unknown>>;
export type OutputArguments<T> = {
    [K in keyof T]: T[K] extends Argument<infer R> ? R : never;
};

export abstract class Argument<T> {
    public description?: string;
    public abstract parse(c: string, client: Connection): Promise<T>;
    public abstract ty(): string;
    public describe(description: string): this {
        this.description = description;
        return this;
    }
}

export class Command<T extends Arguments, F extends Arguments> {
    public constructor(
        public options: CommandOptions<T, F>,
        private e: Exec<T, F>,
    ) {}
    public async usage(): Promise<string> {
        const args = Object.entries(this.options.arguments)
            .map(([name, parser]) => `<${name}:${parser.ty()}>`)
            .join(" ");
        const flags = Object.entries(this.options.flags)
            .map(([name, parser]) => `--${name}:${parser.ty()}`)
            .join(" ");
        return `${this.options.name} ${args} ${flags}`;
    }
    public async parse(
        client: Connection,
        content: string,
    ): Promise<
        { args: OutputArguments<T>; flags: OutputArguments<F> } | undefined
    > {
        content = content.trim();
        if (content.startsWith(client.src.cfg.prefix)) {
            content = content.slice(client.src.cfg.prefix.length);
        } else {
            return;
        }

        let args = content.split(/\s+/g);

        if (args[0]) {
            if (
                args[0].toLowerCase() === this.options.name ||
                this.options.aliases
                    .map((a) => a.toLowerCase())
                    .includes(args[0].toLowerCase())
            ) {
                args.shift();
            } else {
                return;
            }
        }

        // parse flags before positional arguments
        // flags can be of the form --X=Y, --X
        const _ = args.filter((x) => !x.startsWith("--"));
        const flags = args.filter((x) => x.startsWith("--"));

        args = _;

        const f: OutputArguments<F> = {} as never;
        for (const flag of flags) {
            if (flag.startsWith("--")) {
                const [name, value] = flag.slice(2).split("=");
                if (name === undefined || !(name in this.options.flags)) {
                    throw `unknown flag --${name}`;
                }

                f[name as keyof typeof f] = (await this.options.flags[
                    name as keyof typeof this.options.flags
                ]?.parse(value || "true", client)) as never;
            }
        }

        const aa = Object.entries(this.options.arguments);
        const out: OutputArguments<T> = {} as never;

        for (let i = 0; i < aa.length; i++) {
            const used = args[i] || "";
            const [name, parser] = aa[i] as [string, Argument<unknown>];
            try {
                const value = await parser.parse(used, client);
                out[name as keyof typeof out] = value as never;
            } catch (e) {
                throw `error parsing argument '${name}': ${e}`;
            }
        }

        // console.log(out);
        return { args: out, flags: f };
    }

    public async exec(ctx: Context<T, F>): Promise<void> {
        const pl = ctx.client.permissionLevel(ctx.user()._id ?? "");
        const m = ctx.client.permissionMatches(pl, this.options.permission);
        if (!m) {
            throw err.missing_permissions(this.options.permission, pl);
        }

        await this.e(ctx);
    }
}
