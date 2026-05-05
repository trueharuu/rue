import { inspect } from "node:util";
import { ty_assert } from "./util";

export class Tracing {
    private pf: Record<string, number> = {};
    public constructor(public level: Level) {}
    private should(level: Level) {
        const t = [
            Level.Debug,
            Level.Perf,
            Level.Info,
            Level.Warn,
            Level.Error,
            Level.Fatal,
        ];
        return t.indexOf(level) >= t.indexOf(this.level);
    }

    private print(level: Level, contents: Array<unknown>): void {
        if (!this.should(level)) {
            return;
        }

        const label = this.t;

        console.log(
            `\x1b[30m${new Date().toISOString()}\x1b[0m ${
                label ? `${label} ` : ""
            }${this.label(level)} ${contents.map((x) => this.str(x)).join(" ")}`,
        );
    }

    private str(t: unknown): string {
        if (typeof t === "string") {
            return t;
        }
        return inspect(t);
    }

    private label(level: Level): string {
        switch (level) {
            case Level.Debug:
                return "\x1b[34mDEBUG\x1b[0m";
            case Level.Perf:
                return "\x1b[36m PERF\x1b[0m";
            case Level.Info:
                return "\x1b[32m INFO\x1b[0m";
            case Level.Warn:
                return "\x1b[33m WARN\x1b[0m";
            case Level.Error:
                return "\x1b[31mERROR\x1b[0m";
            case Level.Fatal:
                return "\x1b[1;31mFATAL\x1b[0m";
        }
    }

    public debug(...contents: Array<unknown>): void {
        this.print(Level.Debug, contents);
    }

    public info(...contents: Array<unknown>): void {
        this.print(Level.Info, contents);
    }

    public warn(...contents: Array<unknown>): void {
        this.print(Level.Warn, contents);
    }

    public error(...contents: Array<unknown>): void {
        this.print(Level.Error, contents);
    }

    public fatal(...contents: Array<unknown>): void {
        this.print(Level.Fatal, contents);
        process.exit();
    }

    public perf(label: string): void {
        if (label in this.pf) {
            ty_assert(this.pf[label]);
            this.print(Level.Perf, [
                `task ${this.tag(label)} finished in \x1b[33m${
                    Date.now() - this.pf[label]
                }ms\x1b[0m`,
            ]);
            delete this.pf[label];
        } else {
            this.pf[label] = Date.now();
            this.print(Level.Perf, [`task ${this.tag(label)} started`]);
        }
    }

    public tag(s: unknown): string {
        return `\x1b[35m${this.str(s)}\x1b[0m`;
    }

    private t: string | undefined;
    public thread(s: string): Tracing {
        // return a copy
        const t = new Tracing(this.level);
        t.t = s;
        return t;
    }

    public safe<T>(e: T) {
        this.error(e);
    }
}
export enum Level {
    Debug = "DEBUG",
    Perf = "PERF",
    Info = "INFO",
    Warn = "WARN",
    Error = "ERROR",
    Fatal = "FATAL",
}

export const tracing = new Tracing(
    (process.env.TRACING?.toUpperCase() as Level) || Level.Info,
);
