import { describe, expect, it } from "vitest";
import { ARCHIVE_CSP, archiveHeaders, isOneClick, noJsUnsubState, publicPostTarget, seeOther, unsubscribeIntent } from "./public-routes";

describe("seeOther", () => {
  it("redirects with a relative Location, so the scheme behind a TLS proxy stays right", () => {
    const res = seeOther("/join/weekly/check-email");
    expect(res.status).toBe(303);
    expect(res.headers.get("location")).toBe("/join/weekly/check-email");
  });
});

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

describe("unsubscribeIntent", () => {
  const form = (entries: Record<string, string>) => {
    const f = new FormData();
    for (const [k, v] of Object.entries(entries)) f.set(k, v);
    return f;
  };
  it("tells a mail client's one-click from the page's own form posted without JavaScript", () => {
    expect(unsubscribeIntent(form({ "List-Unsubscribe": "One-Click" }))).toBe("one-click");
    expect(unsubscribeIntent(form({ do: "unsubscribe", $ACTION_KEY: "k" }))).toBe("unsubscribe");
    expect(unsubscribeIntent(form({ do: "resubscribe" }))).toBe("resubscribe");
  });
  it("refuses anything else", () => {
    expect(unsubscribeIntent(form({}))).toBeNull();
    expect(unsubscribeIntent(form({ do: "delete" }))).toBeNull();
  });
});

describe("noJsUnsubState", () => {
  it("turns the redirect's ?done= back into the form's state", () => {
    expect(noJsUnsubState(undefined)).toEqual({ step: "ask" });
    expect(noJsUnsubState("unsubscribed")).toEqual({ step: "unsubscribed" });
    expect(noJsUnsubState("resubscribed")).toEqual({ step: "resubscribed" });
    expect(noJsUnsubState("resubscribe_failed")).toMatchObject({ step: "unsubscribed", error: expect.any(String) });
    expect(noJsUnsubState("junk")).toEqual({ step: "ask" });
  });
});

describe("archive headers", () => {
  it("allow no script and only same-site images", () => {
    expect(ARCHIVE_CSP).toBe("default-src 'none'; img-src 'self'; style-src 'unsafe-inline'");
    const h = archiveHeaders();
    expect(h["content-security-policy"]).toBe(ARCHIVE_CSP);
    expect(h["x-content-type-options"]).toBe("nosniff");
    expect(h["content-type"]).toBe("text/html; charset=utf-8");
  });
});
