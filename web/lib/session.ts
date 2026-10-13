import "server-only";
import { cookies } from "next/headers";
import { SESSION_COOKIE } from "./engine";

export async function setSession(token: string, expiresAt: string) {
  (await cookies()).set(SESSION_COOKIE, token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production" && !process.env.ENGINE_URL?.includes("127.0.0.1"),
    sameSite: "lax",
    path: "/",
    expires: new Date(expiresAt),
  });
}

export async function clearSession() {
  (await cookies()).delete(SESSION_COOKIE);
}
