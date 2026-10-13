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
