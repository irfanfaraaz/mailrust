import { expect, test } from "@playwright/test";
import { PNG_1X1, SEED, freshOwner, latestMail, openSignupForm } from "./helpers";

test("a draft with an image and a merge tag autosaves, previews and goes out as a test", async ({ page }) => {
  const owner = await freshOwner(page, "editor");
  await openSignupForm(page, `e2e-ed-${Date.now()}`);
  await page.goto(`/a/${SEED.accountId}/campaigns`);
  await page.getByRole("button", { name: "New campaign" }).click();
  await page.waitForURL(/\/campaigns\/[0-9a-f-]{36}$/);

  await page.getByLabel("Subject").fill("E2E issue one");
  const body = page.getByLabel("Issue body");
  await body.click();
  await page.keyboard.type("/head");
  await page.getByRole("option", { name: /Heading 2/ }).click();
  await page.keyboard.type("Big news");
  await page.keyboard.press("Enter");
  await page.keyboard.type("Hello ");

  await page.getByRole("button", { name: "Merge tag" }).click();
  await page.getByLabel("Fallback").fill("");
  await page.getByRole("button", { name: "Insert" }).click();
  await expect(page.getByRole("alert")).toContainText("fallback");
  await page.getByLabel("Fallback").fill("there");
  await page.getByRole("button", { name: "Insert" }).click();
  await expect(body.getByText("{{first_name | there}}")).toBeVisible();

  await page.getByRole("button", { name: "Image" }).click();
  await page.getByLabel("Image file").setInputFiles({ name: "dot.png", mimeType: "image/png", buffer: PNG_1X1 });
  await page.getByLabel("Alt text").fill("A dot");
  await page.getByRole("button", { name: "Insert" }).click();
  await expect(body.getByRole("img", { name: "A dot" })).toBeVisible();

  await expect(page.getByText("Saved", { exact: true })).toBeVisible({ timeout: 10_000 });
  await page.reload();
  await expect(page.getByLabel("Issue body").getByText("Big news")).toBeVisible();
  await expect(page.getByLabel("Issue body").getByRole("img", { name: "A dot" })).toBeVisible();

  await page.getByRole("button", { name: "Preview" }).click();
  const frame = page.frameLocator('iframe[title="Email preview"]');
  await expect(frame.getByText("Hello there")).toBeVisible();
  await page.getByRole("button", { name: "Phone" }).click();
  await expect(page.locator('iframe[title="Email preview"]')).toHaveAttribute("width", "375");
  await page.keyboard.press("Escape");

  const sent = Date.now();
  await page.getByRole("button", { name: "Send test" }).click();
  await expect(page.getByLabel("Send to")).toHaveValue(owner);
  await page.getByRole("button", { name: "Send test email" }).click();
  await expect(page.getByText("Test sent")).toBeVisible();
  const mail = await latestMail(owner, sent, /Subject: \[Test\] E2E issue one/);
  expect(mail).toContain("This is a test email");
});
