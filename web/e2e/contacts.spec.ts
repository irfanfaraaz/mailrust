import { expect, test } from "@playwright/test";
import { SEED, freshOwner } from "./helpers";

test("a contact is added, edited, unsubscribed and deleted", async ({ page }) => {
  await freshOwner(page, "contacts");
  await page.goto(`/a/${SEED.accountId}/contacts`);
  const email = `c-${Date.now()}@x.test`;
  await page.getByLabel("Email", { exact: true }).fill(email.toUpperCase());
  await page.getByLabel("Tags", { exact: true }).fill("VIP, news");
  await page.getByRole("button", { name: "Add contact" }).click();
  const row = page.getByRole("row", { name: new RegExp(email) });
  await expect(row).toBeVisible();
  await expect(row.getByText("Subscribed")).toBeVisible();

  await row.getByRole("link", { name: email }).click();
  await page.getByLabel("First name").fill("Ana");
  await page.getByLabel("Fields").fill("plan=pro");
  await page.getByRole("button", { name: "Save" }).click();
  await page.reload();
  await expect(page.getByLabel("First name")).toHaveValue("Ana");
  await expect(page.getByLabel("Fields")).toHaveValue("plan=pro");
  await expect(page.getByLabel("Tags", { exact: true })).toHaveValue("news, VIP");

  await page.getByRole("button", { name: "Unsubscribe" }).click();
  await page.getByRole("button", { name: "Unsubscribe contact" }).click();
  await expect(page.getByText("Unsubscribed", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Delete" }).click();
  await page.getByRole("button", { name: "Delete contact" }).click();
  await expect(page).toHaveURL(new RegExp(`/a/${SEED.accountId}/contacts$`));
  await expect(page.getByRole("row", { name: new RegExp(email) })).toHaveCount(0);
});

test("tags are created, renamed and deleted", async ({ page }) => {
  await freshOwner(page, "tags");
  await page.goto(`/a/${SEED.accountId}/tags`);
  const name = `t${Date.now()}`;
  await page.getByLabel("New tag").fill(name);
  await page.getByRole("button", { name: "Add tag" }).click();
  const row = page.getByRole("row", { name: new RegExp(name) });
  await row.getByLabel("Name").fill(`${name}-x`);
  await row.getByRole("button", { name: "Rename" }).click();
  const renamed = page.getByRole("row", { name: new RegExp(`${name}-x`) });
  await expect(renamed).toBeVisible();
  await renamed.getByRole("button", { name: "Delete" }).click();
  await page.getByRole("button", { name: "Delete tag" }).click();
  await expect(page.getByRole("row", { name: new RegExp(name) })).toHaveCount(0);
});
