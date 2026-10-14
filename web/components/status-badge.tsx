import type { Schemas } from "@/lib/engine";
import { Stamp, type Tone } from "./stamp";

export { Stamp, type Tone };

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
