import { describe, expect, it } from "vitest";
import { NextRequest, NextResponse } from "next/server";
import { refreshSessionCookie, SESSION_COOKIE } from "./session-cookie";

const DAY = 24 * 60 * 60 * 1000;

describe("refreshSessionCookie", () => {
  it("pushes the session cookie out to 30 days, keeping its value", () => {
    const req = new NextRequest("http://app.test/a/1/emails", { headers: { cookie: `${SESSION_COOKIE}=mr_sess_abc` } });
    const res = refreshSessionCookie(req, NextResponse.next());
    const c = res.cookies.get(SESSION_COOKIE)!;
    expect(c.value).toBe("mr_sess_abc");
    expect(c.httpOnly).toBe(true);
    expect(c.sameSite).toBe("lax");
    expect(c.path).toBe("/");
    const left = (c.expires as Date).getTime() - Date.now();
    expect(left).toBeGreaterThan(29 * DAY);
    expect(left).toBeLessThanOrEqual(30 * DAY);
  });

  it("sets nothing without a session", () => {
    const res = refreshSessionCookie(new NextRequest("http://app.test/login"), NextResponse.next());
    expect(res.cookies.get(SESSION_COOKIE)).toBeUndefined();
  });

  it("leaves POSTs alone so login and logout actions own the cookie", () => {
    const req = new NextRequest("http://app.test/login", { method: "POST", headers: { cookie: `${SESSION_COOKIE}=mr_sess_old` } });
    expect(refreshSessionCookie(req, NextResponse.next()).cookies.get(SESSION_COOKIE)).toBeUndefined();
  });
});
