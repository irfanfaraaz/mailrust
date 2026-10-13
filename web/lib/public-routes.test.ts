import { describe, expect, it } from "vitest";
import { isOneClick, publicPostTarget } from "./public-routes";

describe("publicPostTarget", () => {
  it("sends plain form posts to the route handlers", () => {
    expect(publicPostTarget("POST", "/join/weekly", false)).toBe("/join/weekly/submit");
    expect(publicPostTarget("POST", "/u/AbC_-12", false)).toBe("/u/AbC_-12/one-click");
    expect(publicPostTarget("POST", "/u/abc/", false)).toBe("/u/abc/one-click");
  });

  it("leaves server actions, other methods and other paths alone", () => {
    expect(publicPostTarget("POST", "/u/abc", true)).toBeNull();
    expect(publicPostTarget("POST", "/join/weekly", true)).toBeNull();
    expect(publicPostTarget("GET", "/u/abc", false)).toBeNull();
    expect(publicPostTarget("POST", "/u/abc/one-click", false)).toBeNull();
    expect(publicPostTarget("POST", "/join/weekly/check-email", false)).toBeNull();
    expect(publicPostTarget("POST", "/confirm/mr_conf_x", false)).toBeNull();
    expect(publicPostTarget("POST", "/login", false)).toBeNull();
  });
});

describe("isOneClick", () => {
  const form = (entries: Record<string, string>) => {
    const f = new FormData();
    for (const [k, v] of Object.entries(entries)) f.set(k, v);
    return f;
  };
  it("accepts only the RFC 8058 body", () => {
    expect(isOneClick(form({ "List-Unsubscribe": "One-Click" }))).toBe(true);
    expect(isOneClick(form({ "List-Unsubscribe": "one-click" }))).toBe(false);
    expect(isOneClick(form({ "list-unsubscribe": "One-Click" }))).toBe(false);
    expect(isOneClick(form({}))).toBe(false);
  });
});
