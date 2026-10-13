const fmt = new Intl.DateTimeFormat("en-GB", { timeZone: "UTC", dateStyle: "medium", timeStyle: "short" });

/** A timestamp in UTC, labelled, so server and viewer agree on what it means. */
export const utc = (iso: string) => `${fmt.format(new Date(iso))} UTC`;

/** A date filter value as ISO, or undefined when it is not a valid date (so a bad URL does not crash the page). */
export function isoDate(v: string): string | undefined {
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? undefined : d.toISOString();
}

/** The engine's signup `closed_reason` as a sentence that says how to open the form; `domains` asks for a link to Domains. Unknown reasons pass through. */
export function closedMessage(reason: string): { text: string; domains?: boolean } {
  const unverified = reason.match(/^(\S+) is not a verified domain of this account$/);
  if (unverified) return { text: `Closed: ${unverified[1]} is not verified, so confirm emails cannot come from it. Verify it, or pick another domain below.`, domains: true };
  const known: Record<string, string> = {
    "signups are turned off": "Closed: signups are turned off. Tick Take signups below and save.",
    "choose a public address for the form": "Closed: choose a public address below.",
    "choose a from address for the confirm email": "Closed: choose the address confirm emails come from.",
    "the account is paused": "Closed: this account is paused, so confirm emails cannot go out.",
  };
  return { text: known[reason] ?? `Closed: ${reason}.` };
}

const n = (x: number) => x.toLocaleString("en-US");

/** "3 contacts", or "Showing 50 of 1,234 contacts" when the list has more than one page. */
export function contactCount(total: number, shown: number, paged: boolean): string {
  if (paged) return `Showing ${n(shown)} of ${n(total)} contacts`;
  return `${n(total)} ${total === 1 ? "contact" : "contacts"}`;
}
