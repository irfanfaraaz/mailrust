import { expect, test } from "@playwright/test";
import { SEED, cli, latestLink, loginAs } from "./helpers";

const uniq = () => `${Date.now()}-${Math.floor(Math.random() * 1e6)}`;

test("an owner invites a teammate who joins as a member without team controls", async ({ page, browser }) => {
  // A fresh owner per story: login links are capped per email per hour.
  const owner = `team-${uniq()}@${SEED.domain}`;
  cli(["user", "invite", owner, "--account", SEED.accountId, "--role", "owner"]);
  await loginAs(page, owner);
  await page.goto(`/a/${SEED.accountId}/team`);
  const mate = `mate-${uniq()}@${SEED.domain}`;
  await page.getByLabel("Email").fill(mate);
  await page.getByRole("button", { name: "Send invite" }).click();
  await expect(page.getByRole("row", { name: new RegExp(mate) })).toContainText("Pending");

  const other = await browser.newContext();
  const p2 = await other.newPage();
  await p2.goto(await latestLink(mate, "invite"));
  await p2.getByRole("button", { name: "Accept invite" }).click();
  await expect(p2).toHaveURL(new RegExp(`/a/${SEED.accountId}/overview`));
  await p2.goto(`/a/${SEED.accountId}/team`);
  await expect(p2.getByRole("row", { name: new RegExp(mate) })).toContainText("member");
  await expect(p2.getByRole("button", { name: "Send invite" })).toHaveCount(0);
  await expect(p2.getByRole("button", { name: "Leave account" })).toBeVisible();
  await other.close();
});

test("the last owner is told they cannot step down", async ({ page }) => {
  // Its own account, so this owner really is the only one.
  const created = cli(["account", "create", "--name", `Solo ${uniq()}`]);
  const account = created.match(/account ([0-9a-f-]{36})/)![1];
  const owner = `solo-${uniq()}@${SEED.domain}`;
  cli(["user", "invite", owner, "--account", account, "--role", "owner"]);
  await loginAs(page, owner);
  await page.goto(`/a/${account}/team`);
  const me = page.getByRole("row", { name: new RegExp(owner) });
  await me.getByLabel("Role").selectOption("member");
  await me.getByRole("button", { name: "Save role" }).click();
  await expect(me.getByRole("alert")).toContainText("at least one owner");
});
