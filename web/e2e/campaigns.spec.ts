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

test("an issue goes to one tag in a wave and the report counts it", async ({ page }) => {
  await freshOwner(page, "sender");
  await openSignupForm(page, `e2e-send-${Date.now()}`);
  await page.getByLabel("Postal address").fill("E2E Ltd\n1 Test Street");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Signup form saved")).toBeVisible();

  const tag = `e2e-tag-${Date.now()}`;
  const reader = `reader-${Date.now()}@${SEED.domain}`;
  await page.goto(`/a/${SEED.accountId}/contacts`);
  await page.getByLabel("Email").fill(reader);
  await page.getByLabel("First name").fill("Rita");
  await page.getByLabel("Tags").fill(tag);
  await page.getByRole("button", { name: "Add contact" }).click();
  await expect(page.getByText("Contact saved")).toBeVisible();

  await page.goto(`/a/${SEED.accountId}/campaigns`);
  await page.getByRole("button", { name: "New campaign" }).click();
  await page.waitForURL(/\/campaigns\/[0-9a-f-]{36}$/);
  const url = page.url();
  await page.getByLabel("Subject").fill(`E2E send ${Date.now()}`);
  await page.getByLabel("Issue body").click();
  await page.keyboard.type("Hello ");
  await page.getByRole("button", { name: "Merge tag" }).click();
  await page.getByRole("button", { name: "Insert" }).click();
  await page.keyboard.type(", this is for you.");
  await expect(page.getByText("Saved", { exact: true })).toBeVisible({ timeout: 10_000 });

  await page.getByRole("button", { name: "Send…" }).click();
  await page.getByLabel("Only some tags").check();
  await page.getByRole("checkbox", { name: tag, exact: true }).first().check();
  await expect(page.getByText("One wave: all 1 readers at once.")).toBeVisible();
  await expect(page.getByText("No subscribed readers")).toHaveCount(0);
  const sent = Date.now();
  await page.getByRole("button", { name: "Send now" }).click();
  await expect(page.getByRole("heading", { name: "Report" })).toBeVisible();

  const mail = await latestMail(reader, sent, /Hello Rita, this is for you\./);
  expect(mail).toMatch(/List-Unsubscribe: <http:\/\/localhost:3000\/u\/[\w-]{86}>/);
  expect(mail).toContain("List-Unsubscribe-Post: List-Unsubscribe=One-Click");

  await expect(async () => {
    await page.goto(url);
    await expect(page.getByTestId("total-sent")).toHaveText("1");
    await expect(page.getByText("Sent", { exact: true }).first()).toBeVisible();
  }).toPass({ timeout: 60_000 });
});
