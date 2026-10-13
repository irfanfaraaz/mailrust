import { spawn } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { ENGINE_ENV, OUTBOX } from "./env";
import { cli } from "./helpers";

const HEALTH = "http://127.0.0.1:8080/health";

async function up() {
  return fetch(HEALTH).then((r) => r.ok, () => false);
}

// Playwright starts `webServer` entries before globalSetup, so the engine (which needs the
// seeded SYSTEM_ACCOUNT_ID) is started here instead, and stopped by the returned teardown.
export default async function globalSetup() {
  if (await up()) throw new Error("something already listens on :8080; stop it before running e2e");
  rmSync(OUTBOX, { recursive: true, force: true });
  mkdirSync(OUTBOX, { recursive: true });
  cli(["migrate"]);
  const created = cli(["account", "create", "--name", `E2E ${Date.now()}`]);
  const accountId = created.match(/account ([0-9a-f-]{36})/)![1];
  const domain = `e2e${Date.now()}.test`;
  cli(["domain", "add", "--account", accountId, domain]); // the fake sender verifies at once
  const ownerEmail = `owner-${Date.now()}@${domain}`;
  cli(["user", "invite", ownerEmail, "--account", accountId, "--role", "owner"]);
  writeFileSync(path.join(__dirname, ".seed.json"), JSON.stringify({ accountId, ownerEmail, domain }));

  const engine = spawn("cargo", ["run", "-q", "--", "serve"], {
    cwd: path.resolve(__dirname, "../.."),
    env: { ...ENGINE_ENV, SYSTEM_ACCOUNT_ID: accountId, SYSTEM_FROM: `login@${domain}` },
    stdio: ["ignore", "ignore", "inherit"],
  });
  let exited = false;
  engine.on("exit", () => (exited = true));
  const deadline = Date.now() + 300_000;
  while (!(await up())) {
    if (exited) throw new Error("the engine exited before /health answered");
    if (Date.now() > deadline) throw new Error("the engine did not answer /health in 5 minutes");
    await new Promise((r) => setTimeout(r, 500));
  }
  return async () => {
    engine.kill("SIGINT");
    await new Promise((r) => (exited ? r(undefined) : engine.once("exit", r)));
  };
}
