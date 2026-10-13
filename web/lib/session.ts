import "server-only";
import { cookies } from "next/headers";
import { SESSION_COOKIE, sessionCookieOptions } from "./session-cookie";

export async function setSession(token: string, expiresAt: string) {
  (await cookies()).set(SESSION_COOKIE, token, sessionCookieOptions(new Date(expiresAt)));
}

export async function clearSession() {
  (await cookies()).delete(SESSION_COOKIE);
}
