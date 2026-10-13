/** The join form's honeypot input. Not a name or autocomplete value browsers autofill, so only bots fill it. */
export const HONEYPOT = "hp_f3a1";

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** The HTML a writer pastes into their own site: a plain form that posts to /join/<slug> (no script). */
export function embedSnippet(origin: string, slug: string, name: string): string {
  const action = `${origin.replace(/\/+$/, "")}/join/${encodeURIComponent(slug)}`;
  return [
    `<form action="${esc(action)}" method="post">`,
    `  <label>Email <input type="email" name="email" required></label>`,
    `  <label>First name <input type="text" name="first_name"></label>`,
    `  <input type="text" name="${HONEYPOT}" tabindex="-1" autocomplete="nope" aria-hidden="true" style="position:absolute;left:-9999px">`,
    `  <button type="submit">Subscribe to ${esc(name)}</button>`,
    `</form>`,
  ].join("\n");
}
