export enum Emoji {
    Checkmark = "✅",
    X = "❌",
    Warning = "⚠️",
}

export function assert<T>(t: T): asserts t is NonNullable<T> {
    if (t === null || t === undefined) {
        throw new Error("Assertion failed: value is null or undefined");
    }
}

export function table<T extends object>(data: T): string {
    let result = "";
    const keys = Object.keys(data);
    const values = Object.values(data);
    const key_header = "key";
    const value_header = "value";

    const key_width = Math.max(...keys.map((k) => k.length), key_header.length);
    const value_width = Math.max(
        ...values.map((v) => String(v).length),
        value_header.length,
    );

    result += `${key_header.padStart(key_width)} | ${value_header.padEnd(value_width)}\n`;
    result += `${"-".repeat(key_width)}-|-${"-".repeat(value_width)}\n`;

    for (const key of keys) {
        const value = String(data[key as keyof T]);
        result += `${key.padStart(key_width)} | ${value.padEnd(value_width)}\n`;
    }

    return result;
}

export function ty_assert<T>(t: T): asserts t is NonNullable<T> {}
