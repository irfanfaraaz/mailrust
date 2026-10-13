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
