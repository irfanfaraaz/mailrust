import { expect, test } from "@playwright/test";
import { SEED, freshOwner, latestLink, loginAs, openSignupForm, randomIp, unsubscribeToken } from "./helpers";

test("a reader joins, confirms, unsubscribes and comes back", async ({ page, request }) => {
  await page.setExtraHTTPHeaders({ "x-forwarded-for": randomIp() });
  const owner = await freshOwner(page, "news");
  const slug = `e2e-${Date.now()}`;
  await openSignupForm(page, slug);
  await page.context().clearCookies(); // the reader is someone else

  const reader = `reader-${Date.now()}@${SEED.domain}`;
  await page.goto(`/join/${slug}`);
  await expect(page.getByRole("heading", { name: "Subscribe to E2E Weekly" })).toBeVisible();
  await page.getByLabel("Email").fill(reader.toUpperCase());
  const sent = Date.now();
  await page.getByRole("button", { name: "Subscribe" }).click();
  await expect(page).toHaveURL(new RegExp(`/join/${slug}/check-email$`));
  await page.goto(await latestLink(reader, "confirm", sent));
  await page.getByRole("button", { name: "Confirm" }).click();
  await expect(page.getByText("You're subscribed to E2E Weekly.")).toBeVisible();

  await loginAs(page, owner);
  await page.goto(`/a/${SEED.accountId}/contacts?q=${encodeURIComponent(reader)}`);
  const row = page.getByRole("row", { name: new RegExp(reader) });
  await expect(row.getByText("Subscribed")).toBeVisible();
  await row.getByRole("link", { name: reader }).click();
  const contactId = page.url().split("/").pop()!;
  const token = unsubscribeToken(contactId);

  await page.goto(`/u/${token}`);
  await page.getByRole("button", { name: "Unsubscribe" }).click();
  await expect(page.getByText("You're unsubscribed from E2E Weekly.")).toBeVisible();
  await page.getByRole("button", { name: "Resubscribe" }).click();
  await expect(page.getByText("You're subscribed to E2E Weekly again.")).toBeVisible();

  // RFC 8058 one-click, the way Gmail sends it: a plain urlencoded POST with no page and no Next-Action.
  expect((await request.post(`/u/${token}`, { form: { hello: "x" } })).status()).toBe(400);
  const oneClick = await request.post(`/u/${token}`, { form: { "List-Unsubscribe": "One-Click" } });
  expect(oneClick.status()).toBe(200);
  await page.goto(`/a/${SEED.accountId}/contacts/${contactId}`);
  await expect(page.getByText("Unsubscribed", { exact: true })).toBeVisible();

  // A broken link still reads as done.
  await page.goto(`/u/${token.slice(0, -2)}`);
  await expect(page.getByText("You're unsubscribed.")).toBeVisible();
});

test("the embed form posts from another site, and a closed form says so", async ({ page, request }) => {
  const ip = randomIp();
  await page.setExtraHTTPHeaders({ "x-forwarded-for": ip });
  await freshOwner(page, "embed");
  const slug = `e2e-embed-${Date.now()}`;
  await openSignupForm(page, slug);

  const headers = { origin: "https://someone-else.test", "x-forwarded-for": ip };
  const ok = await request.post(`/join/${slug}`, { form: { email: `embed-${Date.now()}@${SEED.domain}` }, headers, maxRedirects: 0 });
  expect(ok.status()).toBe(303);
  expect(ok.headers().location).toMatch(new RegExp(`/join/${slug}/check-email$`));
  const bad = await request.post(`/join/${slug}`, { form: { email: "nope" }, headers, maxRedirects: 0 });
  expect(bad.headers().location).toMatch(/\?error=email$/);

  await page.goto(`/a/${SEED.accountId}/signup-form`);
  await page.getByLabel("Take signups").uncheck();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Closed: signups are turned off.")).toBeVisible();
  await page.goto(`/join/${slug}`);
  await expect(page.getByText("This newsletter is not taking signups right now.")).toBeVisible();
  await page.goto("/join/no-such-newsletter-here");
  // notFound() inside Suspense streams Next's 404 page (the status may already be 200).
  await expect(page.getByText("This page could not be found.")).toBeVisible();
});
