const fmt = new Intl.DateTimeFormat("en-GB", { timeZone: "UTC", dateStyle: "medium", timeStyle: "short" });

/** A timestamp in UTC, labelled, so server and viewer agree on what it means. */
export const utc = (iso: string) => `${fmt.format(new Date(iso))} UTC`;
