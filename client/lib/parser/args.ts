import type { Connection } from "../connection";
import { Argument } from "./handler";

export function word(): Argument<string> {
    return new (class Word extends Argument<string> {
        public async parse(c: string, _client: Connection): Promise<string> {
            if (c === "") {
                throw "expected a word";
            }

            return c;
        }

        public ty(): string {
            return "word";
        }
    })();
}

export function optional<T>(parser: Argument<T>): Argument<T | null> {
    return new (class extends Argument<T | null> {
        public async parse(c: string, client: Connection): Promise<T | null> {
            if (c === "") {
                return null;
            }
            return await parser.parse(c, client);
        }

        public ty(): string {
            return `${parser.ty()}?`;
        }
    })();
}

export function bool(): Argument<boolean> {
    return new (class Bool extends Argument<boolean> {
        public async parse(c: string, _client: Connection): Promise<boolean> {
            if (c.toLowerCase() === "true") {
                return true;
            } else if (c.toLowerCase() === "false") {
                return false;
            } else {
                throw "expected a boolean (true/false)";
            }
        }

        public ty(): string {
            return "bool";
        }
    })();
}

export function enumerable<T>(t: T): Argument<T[keyof T]> {
    return new (class Enum extends Argument<T[keyof T]> {
        public async parse(
            c: string,
            _client: Connection,
        ): Promise<T[keyof T]> {
            const values = Object.values(t as object) as unknown as string[];

            for (const v of values) {
                if (v.toLowerCase() === c.toLowerCase()) {
                    return v as unknown as T[keyof T];
                }
            }

            throw `expected one of ${values.join(", ")}`;
        }

        public ty(): string {
            return "enum";
        }
    })();
}

export function int(n?: number, m?: number): Argument<number> {
    return new (class Num extends Argument<number> {
        public async parse(c: string, _client: Connection): Promise<number> {
            const x = Number.parseInt(c, 10);
            if (Number.isNaN(x)) {
                throw "expected an integer";
            }

            // min=None max=m
            if (m === undefined && n !== undefined) {
                if (x > n) {
                    throw `expected an integer in (-∞, ${n}]`;
                }
            }

            // min=n max=None
            else if (n === undefined && m !== undefined) {
                if (x < m) {
                    throw `expected an integer in [${m}, ∞)`;
                }
            }

            // min=n max=m
            else if (n !== undefined && m !== undefined) {
                if (x < n || x > m) {
                    throw `expected an integer in [${n}, ${m}]`;
                }
            }

            return x;
        }

        public ty(): string {
            return "int";
        }
    })();
}

export function float(n?: number, m?: number): Argument<number> {
    return new (class Num extends Argument<number> {
        public async parse(c: string, _client: Connection): Promise<number> {
            const x = Number.parseFloat(c);
            if (Number.isNaN(x)) {
                throw "expected an float";
            }

            // min=None max=m
            if (m === undefined && n !== undefined) {
                if (x > n) {
                    throw `expected an float in (-∞, ${n}]`;
                }
            }

            // min=n max=None
            else if (n === undefined && m !== undefined) {
                if (x < m) {
                    throw `expected an float in [${m}, ∞)`;
                }
            }

            // min=n max=m
            else if (n !== undefined && m !== undefined) {
                if (x < n || x > m) {
                    throw `expected an float in [${n}, ${m}]`;
                }
            }

            return x;
        }

        public ty(): string {
            return "int";
        }
    })();
}
