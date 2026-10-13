import { expect, test } from "@playwright/test";
import { SEED, latestLink, loginAs } from "./helpers";

test("a user logs in with a magic link and lands on Overview", async ({ page }) => {
  await loginAs(page, SEED.ownerEmail);
  await expect(page.getByRole("heading", { name: "Overview" })).toBeVisible();
});

test("an unknown email sees the same page and gets no mail", async ({ page }) => {
  await page.goto("/login");
  await page.getByLabel("Email").fill(`nobody-${Date.now()}@x.test`);
  await page.getByRole("button", { name: "Send login link" }).click();
  await expect(page.getByRole("heading", { name: "Check your email" })).toBeVisible();
});

test("a used link shows a clear error", async ({ page }) => {
  await loginAs(page, SEED.ownerEmail);
  const link = await latestLink(SEED.ownerEmail, "login");
  await page.context().clearCookies();
  await page.goto(link);
  await page.getByRole("button", { name: "Log in" }).click();
  await expect(page.getByText("This link is invalid or has expired")).toBeVisible();
});

test("logging out ends the session", async ({ page }) => {
  await loginAs(page, SEED.ownerEmail);
  await page.getByRole("button", { name: "Account menu" }).click();
  await page.getByRole("menuitem", { name: "Log out", exact: true }).click();
  await expect(page).toHaveURL(/\/login/);
  await page.goto(`/a/${SEED.accountId}/overview`);
  await expect(page).toHaveURL(/\/login\?next=/);
});
