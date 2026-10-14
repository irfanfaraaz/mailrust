import type { Schemas } from "@/lib/engine";

export type Tone = "good" | "wait" | "bad" | "info" | "quiet";

// One colour language for every status: green done, amber waiting or delayed, red needs attention, navy in progress.
const tones: Record<Tone, string> = {
  good: "text-good",
  wait: "text-wait",
  bad: "text-destructive",
  info: "text-primary",
  quiet: "text-muted-foreground",
};

/** A rubber stamp: mono uppercase, double rule, a slight tilt. */
const stamp =
  "inline-flex w-fit shrink-0 -rotate-2 items-center rounded-[2px] border-[1.5px] border-current px-1.5 py-px font-mono text-[0.6875rem] font-bold tracking-[0.08em] whitespace-nowrap uppercase outline-1 outline-offset-[1.5px] outline-current/40";

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

/** A rubber-stamp label in one of the status tones. */
export function Stamp({ tone, title, children }: { tone: Tone; title?: string; children: React.ReactNode }) {
  return <span className={`${stamp} ${tones[tone]}`} title={title}>{children}</span>;
}

function StatusBadge({ tone, status, title }: { tone: Tone; status: string; title?: string }) {
  return <Stamp tone={tone} title={title}>{statusLabel(status)}</Stamp>;
}

/** Email status is a string on some views (detail adds "scheduled"), so unknown values fall back to a quiet badge. */
export function EmailStatusBadge({ status }: { status: string }) {
  return <StatusBadge tone={emailTone[status] ?? "quiet"} status={status} />;
}

export function DomainStatusBadge({ status }: { status: Schemas["DomainStatus"] }) {
  return <StatusBadge tone={domainTone[status]} status={status} />;
}

const contactTone: Record<Schemas["ContactStatus"], Tone> = { subscribed: "good", pending: "wait", unsubscribed: "quiet" };

/** What "pending" means, for the badge tooltip and the contact page. */
export const PENDING_HINT = "Waiting for them to click the link in the confirm email. Nothing is sent to them until then.";

export function ContactStatusBadge({ status }: { status: Schemas["ContactStatus"] }) {
  return <StatusBadge tone={contactTone[status]} status={status} title={status === "pending" ? PENDING_HINT : undefined} />;
}

const importTone: Record<Schemas["ImportStatus"], Tone> = { done: "good", queued: "wait", running: "info", failed: "bad" };

export function ImportStatusBadge({ status }: { status: Schemas["ImportStatus"] }) {
  return <StatusBadge tone={importTone[status]} status={status} />;
}

/** On the suppression list: nothing is sent to this address. */
export function BlockedBadge() {
  return <StatusBadge tone="bad" status="blocked" />;
}

const campaignTone: Record<Schemas["CampaignStatus"], Tone> = { draft: "quiet", scheduled: "info", sending: "info", paused: "wait", sent: "good", cancelled: "quiet" };

export function CampaignStatusBadge({ status }: { status: Schemas["CampaignStatus"] }) {
  return <StatusBadge tone={campaignTone[status]} status={status} />;
}
