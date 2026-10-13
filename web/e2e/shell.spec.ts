import { expect, test } from "@playwright/test";
import { SEED, cli, loginAs } from "./helpers";

/** A new member of the seed account: login links are capped at 5 per email per hour, and login.spec already uses the seed owner. */
function freshUser() {
  const email = `user-${Date.now()}@${SEED.domain}`;
  cli(["user", "invite", email, "--account", SEED.accountId, "--role", "owner"]);
  return email;
}

test("the sidebar groups the door 1 pages and the shared pages", async ({ page }) => {
  await loginAs(page, freshUser());
  const nav = page.getByRole("navigation", { name: "Main" });
  await expect(nav.getByText("Sending API")).toBeVisible();
  await expect(nav.getByText("Newsletter")).toBeVisible();
  await expect(nav.getByText("Shared")).toBeVisible();
  for (const name of ["Overview", "Emails", "API keys", "Webhooks", "Contacts", "Tags", "Signup form", "Domains", "Suppressions", "Team"]) {
    await expect(nav.getByRole("link", { name })).toBeVisible();
  }
});

test("someone else's account id shows not found", async ({ page }) => {
  await loginAs(page, freshUser());
  await page.goto("/a/00000000-0000-7000-8000-000000000000/overview");
  await expect(page.getByText("Not found")).toBeVisible();
});

test("a paused account shows a red banner with the reason", async ({ page }) => {
  // Log in first: the seed account also sends the login emails, and a paused account holds them.
  await loginAs(page, freshUser());
  cli(["account", "pause", SEED.accountId, "--reason", "bounce rate test"]);
  try {
    await page.reload();
    // Next's route announcer is also role="alert".
    await expect(page.getByRole("alert").filter({ hasText: "Sending is paused" })).toContainText("bounce rate test");
  } finally {
    cli(["account", "resume", SEED.accountId]);
  }
});

test("the account switcher lists my accounts", async ({ page }) => {
  const second = cli(["account", "create", "--name", `Second ${Date.now()}`]).match(/account ([0-9a-f-]{36})/)![1];
  const email = freshUser();
  cli(["user", "invite", email, "--account", second, "--role", "member"]);
  await loginAs(page, email);
  await page.getByRole("button", { name: "Switch account" }).click();
  await page.getByRole("menuitem", { name: /Second/ }).click();
  await expect(page).toHaveURL(new RegExp(`/a/${second}/overview`));
});
