import { expect, test } from "@playwright/test";
import { SEED, cli, loginAs } from "./helpers";

/** A fresh owner per story: login links are capped per email per hour. */
async function freshOwner(page: Parameters<typeof loginAs>[0]) {
  const email = `domains-${Date.now()}-${Math.floor(Math.random() * 1e6)}@${SEED.domain}`;
  cli(["user", "invite", email, "--account", SEED.accountId, "--role", "owner"]);
  await loginAs(page, email);
}

test("adding a domain shows the DNS records to create", async ({ page }) => {
  await freshOwner(page);
  await page.goto(`/a/${SEED.accountId}/domains`);
  const name = `news${Date.now()}.test`;
  await page.getByLabel("Domain", { exact: true }).fill(name);
  await page.getByRole("button", { name: "Add domain" }).click();
  await expect(page.getByRole("heading", { name, exact: true })).toBeVisible();
  const records = page.getByRole("table", { name: "DNS records" });
  await expect(records.getByRole("row")).not.toHaveCount(1);
  await expect(page.getByRole("button", { name: "Check now" })).toBeVisible();
});

test("a domain can be removed", async ({ page }) => {
  await freshOwner(page);
  await page.goto(`/a/${SEED.accountId}/domains`);
  const name = `gone${Date.now()}.test`;
  await page.getByLabel("Domain", { exact: true }).fill(name);
  await page.getByRole("button", { name: "Add domain" }).click();
  await expect(page.getByRole("heading", { name, exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Remove domain" }).click();
  await page.getByRole("button", { name: "Remove domain" }).last().click();
  await expect(page).toHaveURL(/\/domains$/);
  await expect(page.getByRole("link", { name })).toHaveCount(0);
});
