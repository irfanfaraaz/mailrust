import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { SEED, freshOwner } from "./helpers";

test("a CSV imports with a report, and the list exports", async ({ page }) => {
  await freshOwner(page, "import");
  const stamp = Date.now();
  // BOM, CRLF, a quoted comma, a bad row, and the same person again in capitals.
  const csv = `﻿Email,Name,Tags\r\nimp-${stamp}@x.test,"Lopez, Ana",e2e\r\nnot-an-email,Bob,\r\nIMP-${stamp}@X.TEST,Ana Lopez,\r\n`;
  await page.goto(`/a/${SEED.accountId}/contacts/import`);
  await page.getByLabel("CSV file").setInputFiles({ name: "contacts.csv", mimeType: "text/csv", buffer: Buffer.from(csv, "utf8") });
  await page.getByLabel("These people agreed to get my emails").check();
  await page.getByRole("button", { name: "Import" }).click();
  await expect(page.getByRole("heading", { name: "Import done" })).toBeVisible({ timeout: 30_000 });
  await expect(page.getByText("Added 1")).toBeVisible();
  await expect(page.getByText("Updated 1")).toBeVisible();
  await expect(page.getByText("Skipped 1")).toBeVisible();
  await expect(page.getByRole("row", { name: /3 .*not-an-email is not a valid email address/ })).toBeVisible();

  await page.goto(`/a/${SEED.accountId}/contacts`);
  const download = page.waitForEvent("download");
  await page.getByRole("link", { name: "Export CSV" }).click();
  const text = readFileSync((await (await download).path())!, "utf8");
  expect(text.split("\n")[0]).toMatch(/^email,first_name,last_name,status,tags,created_at/);
  expect(text).toContain(`imp-${stamp}@x.test,Ana,Lopez,subscribed,e2e,`);
});

test("a file without an email column is refused with the reason", async ({ page }) => {
  await freshOwner(page, "import-consent");
  await page.goto(`/a/${SEED.accountId}/contacts/import`);
  await page.getByLabel("CSV file").setInputFiles({ name: "c.csv", mimeType: "text/csv", buffer: Buffer.from("name\nAna\n") });
  await page.getByLabel("These people agreed to get my emails").check();
  await page.getByRole("button", { name: "Import" }).click();
  await expect(page.getByRole("alert").filter({ hasText: "email column" })).toBeVisible();
});
