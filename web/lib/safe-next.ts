/** A redirect target after login: only a path on this site, else "/". */
export function safeNext(next: string | null | undefined): string {
  // Browsers drop tab/newline and treat "\" as "/" in URLs, so "/\t/evil.com" would become "//evil.com".
  if (!next || !next.startsWith("/") || next.startsWith("//") || /[\s\\\x00-\x1f\x7f]/.test(next)) return "/";
  return next;
}
