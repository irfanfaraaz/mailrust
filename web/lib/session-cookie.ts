import type { NextRequest, NextResponse } from "next/server";

export const SESSION_COOKIE = "mr_session";
/** The engine slides a session to 30 days each time it is used. */
const SESSION_DAYS = 30;

export function sessionCookieOptions(expires: Date) {
  return {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production" && !process.env.ENGINE_URL?.includes("127.0.0.1"),
    sameSite: "lax" as const,
    path: "/",
    expires,
  };
}

/**
 * On page navigation (GET), re-sets the session cookie with a fresh 30-day expiry so it lives as
 * long as the engine session. POSTs are server actions, which set or delete the cookie themselves.
 */
export function refreshSessionCookie(req: NextRequest, res: NextResponse): NextResponse {
  const token = req.cookies.get(SESSION_COOKIE)?.value;
  if (token && req.method === "GET") res.cookies.set(SESSION_COOKIE, token, sessionCookieOptions(new Date(Date.now() + SESSION_DAYS * 864e5)));
  return res;
}
