import { spawnSync } from "node:child_process";
import { buildUrl } from "../pg-common";

const setup = spawnSync(process.execPath, ["x", "prisma", "generate"], {
  cwd: import.meta.dir,
  env: { ...process.env, DATABASE_URL: process.env.DATABASE_URL ?? buildUrl() },
  stdio: "inherit",
});

if (setup.status !== 0) {
  throw new Error(`prisma generate failed with status ${setup.status}`);
}

await import("./server.ts");
