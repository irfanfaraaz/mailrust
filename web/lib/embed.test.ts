import { describe, expect, it } from "vitest";
import { embedSnippet } from "./embed";

describe("embedSnippet", () => {
  it("posts a plain form to the join route", () => {
    const html = embedSnippet("https://app.shop.com/", "weekly", "Shop Weekly");
    expect(html).toContain('<form action="https://app.shop.com/join/weekly" method="post">');
    expect(html).toContain('name="email"');
    expect(html).toContain('name="first_name"');
    expect(html).toMatch(/name="website"[^>]*tabindex="-1"/);
    expect(html).toContain("Subscribe to Shop Weekly");
  });

  it("escapes the newsletter name", () => {
    const html = embedSnippet("https://a.test", "x-y", `"><script>alert(1)</script>`);
    expect(html).not.toContain("<script>");
    expect(html).toContain("&quot;&gt;&lt;script&gt;");
  });
});
