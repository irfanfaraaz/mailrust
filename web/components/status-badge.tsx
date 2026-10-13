import { Badge } from "@/components/ui/badge";
import type { Schemas } from "@/lib/engine";

type Tone = "good" | "wait" | "bad" | "info" | "quiet";

// One colour language for every status: green done, amber waiting or delayed, red needs attention.
const tones: Record<Tone, string> = {
  good: "border-transparent bg-emerald-500/12 text-emerald-800 dark:bg-emerald-400/15 dark:text-emerald-300",
  wait: "border-transparent bg-amber-500/15 text-amber-800 dark:bg-amber-400/15 dark:text-amber-300",
  bad: "border-transparent bg-destructive/12 text-destructive dark:bg-destructive/20",
  info: "border-transparent bg-secondary text-secondary-foreground",
  quiet: "border-border bg-transparent text-muted-foreground",
};

const emailTone: Record<string, Tone> = {
  delivered: "good",
  sent: "good",
  delivery_delayed: "wait",
  queued: "info",
  scheduled: "info",
  sending: "info",
  bounced: "bad",
  complained: "bad",
  failed: "bad",
  suppressed: "quiet",
  canceled: "quiet",
  unknown: "quiet",
};

const domainTone: Record<Schemas["DomainStatus"], Tone> = { verified: "good", pending: "wait", failed: "bad" };

/** "delivery_delayed" -> "Delivery delayed" */
export const statusLabel = (s: string) => (s.charAt(0).toUpperCase() + s.slice(1)).replace(/_/g, " ");

function StatusBadge({ tone, status }: { tone: Tone; status: string }) {
  return <Badge variant="outline" className={tones[tone]}>{statusLabel(status)}</Badge>;
}

/** Email status is a string on some views (detail adds "scheduled"), so unknown values fall back to a quiet badge. */
export function EmailStatusBadge({ status }: { status: string }) {
  return <StatusBadge tone={emailTone[status] ?? "quiet"} status={status} />;
}

export function DomainStatusBadge({ status }: { status: Schemas["DomainStatus"] }) {
  return <StatusBadge tone={domainTone[status]} status={status} />;
}
