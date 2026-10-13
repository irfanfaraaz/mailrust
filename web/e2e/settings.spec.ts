import { expect, test } from "@playwright/test";
import { SEED, cli, loginAs } from "./helpers";

/** A fresh owner per story: login links are capped per email per hour. */
async function freshOwner(page: Parameters<typeof loginAs>[0]) {
  const email = `settings-${Date.now()}-${Math.floor(Math.random() * 1e6)}@${SEED.domain}`;
  cli(["user", "invite", email, "--account", SEED.accountId, "--role", "owner"]);
  await loginAs(page, email);
}

test("an API key is shown once, then can be deleted", async ({ page }) => {
  await freshOwner(page);
  await page.goto(`/a/${SEED.accountId}/api-keys`);
  const name = `key-${Date.now()}`;
  await page.getByLabel("Name").fill(name);
  await page.getByRole("button", { name: "Create key" }).click();
  await expect(page.getByText(/mr_live_[\w-]{20,}/)).toBeVisible();
  await expect(page.getByText("You will not see this key again")).toBeVisible();
  await page.reload();
  await expect(page.getByText(/mr_live_[\w-]{20,}/)).toHaveCount(0);
  const row = page.getByRole("row", { name: new RegExp(name) });
  await row.getByRole("button", { name: "Delete" }).click();
  await page.getByRole("button", { name: "Delete key" }).click();
  await expect(page.getByRole("row", { name: new RegExp(name) })).toHaveCount(0);
});

test("a webhook is added with its signing secret shown once", async ({ page }) => {
  await freshOwner(page);
  await page.goto(`/a/${SEED.accountId}/webhooks`);
  await page.getByLabel("URL").fill("https://example.com/hook");
  await page.getByLabel("delivered").check();
  await page.getByRole("button", { name: "Add webhook" }).click();
  await expect(page.getByText("You will not see this secret again")).toBeVisible();
  await expect(page.getByRole("row", { name: /example\.com\/hook/ }).first()).toBeVisible();
});

test("suppressions can be added, searched and removed", async ({ page }) => {
  await freshOwner(page);
  await page.goto(`/a/${SEED.accountId}/suppressions`);
  const addr = `blocked-${Date.now()}@x.test`;
  await page.getByLabel("Address").fill(addr);
  await page.getByRole("button", { name: "Block address" }).click();
  await page.getByLabel("Search").fill(addr.slice(0, 12));
  await page.getByRole("button", { name: "Search" }).click();
  const row = page.getByRole("row", { name: new RegExp(addr) });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "Unblock" }).click();
  await page.getByRole("button", { name: "Unblock address" }).click();
  await expect(page.getByRole("row", { name: new RegExp(addr) })).toHaveCount(0);
});
