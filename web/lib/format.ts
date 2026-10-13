const fmt = new Intl.DateTimeFormat("en-GB", { timeZone: "UTC", dateStyle: "medium", timeStyle: "short" });

/** A timestamp in UTC, labelled, so server and viewer agree on what it means. */
export const utc = (iso: string) => `${fmt.format(new Date(iso))} UTC`;

/** A date filter value as ISO, or undefined when it is not a valid date (so a bad URL does not crash the page). */
export function isoDate(v: string): string | undefined {
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? undefined : d.toISOString();
}
