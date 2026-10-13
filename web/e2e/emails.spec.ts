import { expect, test } from "@playwright/test";
import { SEED, cli, loginAs } from "./helpers";

test("the log finds an email by recipient and shows its story", async ({ page, request }) => {
  const key = cli(["key", "create", "--account", SEED.accountId, "--name", "e2e"]).match(/mr_live_[\w-]+/)![0];
  const to = `reader-${Date.now()}@x.test`;
  const res = await request.post("http://127.0.0.1:8080/emails", {
    headers: { authorization: `Bearer ${key}` },
    data: { from: `hi@${SEED.domain}`, to, subject: "Hello from e2e", text: "hi" },
  });
  expect(res.ok()).toBeTruthy();

  // A fresh member: login links are capped per email per hour.
  const owner = `user-${Date.now()}@${SEED.domain}`;
  cli(["user", "invite", owner, "--account", SEED.accountId, "--role", "owner"]);
  await loginAs(page, owner);
  await page.getByRole("navigation", { name: "Main" }).getByRole("link", { name: "Emails" }).click();
  await page.getByLabel("Search").fill(to);
  await page.getByRole("button", { name: "Filter" }).click();
  const row = page.getByRole("row", { name: /Hello from e2e/ });
  await expect(row).toBeVisible();
  await row.getByRole("link").first().click();
  await expect(page.getByRole("heading", { name: "Hello from e2e" })).toBeVisible();
  await expect(page.getByRole("list", { name: "Events" })).toContainText(/sent|delivered/);
});
