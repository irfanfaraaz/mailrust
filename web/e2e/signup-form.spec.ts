import { expect, test } from "@playwright/test";
import { SEED, freshOwner, openSignupForm } from "./helpers";

test("the signup form is set up, shared and closed again", async ({ page }) => {
  await freshOwner(page, "signup");
  await page.goto(`/a/${SEED.accountId}/signup-form`);
  await page.getByLabel("Public address").fill("ab");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("alert").filter({ hasText: "3-40 characters" })).toBeVisible();

  const slug = `e2e-${Date.now()}`;
  await openSignupForm(page, slug);
  await expect(page.getByRole("link", { name: new RegExp(`/join/${slug}$`) })).toBeVisible();
  await expect(page.getByLabel("Embed code")).toHaveValue(new RegExp(`/join/${slug}" method="post"`));

  await page.getByLabel("Take signups").uncheck();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Closed: signups are turned off.")).toBeVisible();
});

test("the postal address is saved and the form can skip the first name", async ({ page }) => {
  await freshOwner(page, "postal");
  const slug = `e2e-postal-${Date.now()}`;
  await openSignupForm(page, slug);
  await page.getByLabel("Postal address").fill("Shop Ltd\n1 Main Street");
  await page.getByLabel("Ask for first name").uncheck();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Signup form saved")).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Postal address")).toHaveValue("Shop Ltd\n1 Main Street");
  await page.goto(`/join/${slug}`);
  await expect(page.getByLabel("Email")).toBeVisible();
  await expect(page.getByLabel("First name (optional)")).toHaveCount(0);
});
