import { secureCookies } from "./session-cookie";

/** The join form's typed values, kept for one retry after the engine rejects the email. */
export const DRAFT_COOKIE = "join_draft";

export function readDraft(raw: string | undefined): { email: string; first_name: string } {
  try {
    const d = JSON.parse(raw ?? "{}");
    return { email: typeof d.email === "string" ? d.email : "", first_name: typeof d.first_name === "string" ? d.first_name : "" };
  } catch {
    return { email: "", first_name: "" };
  }
}

/** Kept for the retry: short-lived, scoped to this form, never in the URL. */
export function draftCookieOptions(page: string) {
  return { path: page, maxAge: 300, httpOnly: true, sameSite: "lax" as const, secure: secureCookies() };
}

/** Which field a 400 from /public/join is about. The engine checks the email first, and only that message says so. */
export function joinErrorField(message: string): "email" | "name" {
  return message.endsWith("is not a valid email address") ? "email" : "name";
}
