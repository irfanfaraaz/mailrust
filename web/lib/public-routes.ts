import { NextResponse } from "next/server";

/** 303 with a relative Location: built from req.url it would say http:// behind a TLS-terminating proxy. */
export function seeOther(path: string): NextResponse {
  return new NextResponse(null, { status: 303, headers: { location: path } });
}

/**
 * Plain HTML form posts (no server action) to a public page go to the route handler next to it:
 * forms embedded on other sites post to /join/<slug>, and mail clients post RFC 8058 one-click
 * unsubscribes to /u/<token>. A page cannot take those, a route handler can.
 */
export function publicPostTarget(method: string, pathname: string, isAction: boolean): string | null {
  if (method !== "POST" || isAction) return null;
  const m = pathname.match(/^\/(join|u)\/([^/]+)\/?$/);
  if (!m) return null;
  return `/${m[1]}/${m[2]}/${m[1] === "join" ? "submit" : "one-click"}`;
}

/** RFC 8058: the one-click POST body is exactly List-Unsubscribe=One-Click. */
export function isOneClick(form: FormData): boolean {
  return form.get("List-Unsubscribe") === "One-Click";
}

/**
 * What a POST that reached /u/<token>/one-click asks for: a mail client's RFC 8058 one-click, or the
 * page's own form submitted without JavaScript (its server action cannot run, so it lands here too).
 */
export function unsubscribeIntent(form: FormData): "one-click" | "unsubscribe" | "resubscribe" | null {
  if (isOneClick(form)) return "one-click";
  const d = form.get("do");
  return d === "unsubscribe" || d === "resubscribe" ? d : null;
}

export type UnsubStep = { step: "ask" | "unsubscribed" | "resubscribed"; error?: string };

export const RESUBSCRIBE_REFUSED = "This link cannot subscribe you again. Use the newsletter's signup form.";

/** The /u/<token> page state after a no-JavaScript submit redirected back with ?done=. */
export function noJsUnsubState(done: string | undefined): UnsubStep {
  if (done === "unsubscribed" || done === "resubscribed") return { step: done };
  if (done === "resubscribe_failed") return { step: "unsubscribed", error: RESUBSCRIBE_REFUSED };
  return { step: "ask" };
}
