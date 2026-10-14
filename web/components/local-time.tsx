"use client";

import { useSyncExternalStore } from "react";
import { when } from "@/lib/format";

const noop = () => () => {};
const browserZone = () => Intl.DateTimeFormat().resolvedOptions().timeZone;

/** A timestamp in the viewer's time zone. The server does not know it, so the first paint is UTC and the browser swaps it in. */
export function LocalTime({ iso, dateOnly = false, className }: { iso: string; dateOnly?: boolean; className?: string }) {
  const zone = useSyncExternalStore(noop, browserZone, () => "UTC");
  return <time dateTime={iso} className={className}>{when(iso, zone, { dateOnly })}</time>;
}
