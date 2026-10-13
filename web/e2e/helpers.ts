import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { expect, type Page } from "@playwright/test";
import { ENGINE_ENV, OUTBOX } from "./env";

const seedFile = path.join(__dirname, ".seed.json");
/** Written by global setup; empty while global setup itself imports this file. */
export const SEED: { accountId: string; ownerEmail: string; domain: string } = existsSync(seedFile)
  ? JSON.parse(readFileSync(seedFile, "utf8"))
  : { accountId: "", ownerEmail: "", domain: "" };

/** Undo quoted-printable soft breaks and =XX escapes in a raw .eml. */
function decodeQp(s: string) {
  return s.replace(/=\r?\n/g, "").replace(/=([0-9A-F]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16)));
}

/** Newest link of `kind` sent to `email` (in a file written after `since`, ms), waiting up to 15 s for the worker. */
export async function latestLink(email: string, kind: "login" | "invite", since = 0): Promise<string> {
  const re = kind === "login" ? /https?:\/\/\S+\/login\/confirm\?token=mr_login_[\w-]+/ : /https?:\/\/\S+\/invite\?token=mr_inv_[\w-]+/;
  const to = email.toLowerCase();
  let found: string | undefined;
  await expect(async () => {
    const files = readdirSync(OUTBOX).map((f) => path.join(OUTBOX, f)).filter((f) => statSync(f).mtimeMs >= since).sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
    for (const f of files) {
      const raw = decodeQp(readFileSync(f, "utf8"));
      const lower = raw.toLowerCase();
      if ((lower.includes(`to: ${to}`) || lower.includes(`to: <${to}>`)) && re.test(raw)) {
        found = raw.match(re)![0];
        return;
      }
    }
    throw new Error(`no ${kind} link for ${email} yet`);
  }).toPass({ timeout: 15_000 });
  return found!;
}

export async function loginAs(page: Page, email: string) {
  await page.goto("/login");
  await page.getByLabel("Email").fill(email);
  const sent = Date.now(); // an older (used) link must not be picked up
  await page.getByRole("button", { name: "Send login link" }).click();
  await expect(page.getByRole("heading", { name: "Check your email" })).toBeVisible();
  await page.goto(await latestLink(email, "login", sent));
  await page.getByRole("button", { name: "Log in" }).click();
  await expect(page).toHaveURL(/\/a\/[0-9a-f-]+\/overview/);
}

export function cli(args: string[]) {
  return execFileSync("cargo", ["run", "-q", "--", ...args], {
    cwd: path.resolve(__dirname, "../.."),
    env: ENGINE_ENV,
    encoding: "utf8",
  });
}
