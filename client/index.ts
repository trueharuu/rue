import { Main, type Cfg } from "./lib/connection";
import { config } from "dotenv";
config();
console.log(process.env.TOKEN);
const cfg: Cfg = {
    admins: process.env.ADMINS?.split(",") || [],
    dev_id: process.env.DEV_ID,
    prefix: process.env.PREFIX || "!",
    token: process.env.TOKEN || "",
};
const main = new Main(cfg);

await main.spawn_cluster();

process.on("SIGINT", async () => {
    for (const con of main.connections.values()) {
        await con.disconnect();
    }

    process.exit();
});

await main.connect();
