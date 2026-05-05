import type { Types } from "@haelp/teto";
import type { Connection } from "./connection";
import type { OutputArguments } from "./parser/handler";

export class Context<T, F> {
    public constructor(
        public readonly client: Connection,
        public readonly message: Types.Events.in.Room["room.chat"],
        public readonly args: OutputArguments<T>,
        public readonly flags: OutputArguments<F>,
        public readonly id: string = Bun.randomUUIDv7(),
    ) {}

    public user() {
        return this.message.user;
    }

    public async reply(text: string) {
        return await this.client.room.chat(text);
    }
}
