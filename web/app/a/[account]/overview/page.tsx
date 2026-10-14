import { PlusIcon } from "lucide-react";
import Link from "next/link";
import { Suspense } from "react";
import { cn } from "cn";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { CampaignStatusBadge, Stamp, type Tone } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardHeader } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { createCampaign } from "../campaigns/actions";

export const metadata = { title: "Overview" };

const pct = (n: number) => `${(n * 100).toFixed(2)}%`;
const num = new Intl.NumberFormat("en");
/** Share of `part` in `whole` as a percentage, or a dash when nothing went out yet. */
const share = (part: number, whole: number) => (whole > 0 ? `${((part / whole) * 100).toFixed(1)}%` : "–");

type Props = { params: Promise<{ account: string }> };

export default function Overview(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading rows={5} />}>
        <Stats {...props} />
      </Suspense>
    </div>
  );
}

/** The newest issue at a glance: what it is, where it stands, how it did. */
function LatestIssue({ account, c }: { account: string; c: Schemas["CampaignSummary"] | undefined }) {
  return (
    <Card>
      <CardHeader>
        <h2 className="text-lg leading-snug font-semibold">Latest issue</h2>
        <CardAction>
          <Link href={`/a/${account}/campaigns`} className="text-sm text-primary underline-offset-2 hover:underline">All campaigns</Link>
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {c ? (
          <>
            <div className="flex items-start justify-between gap-3">
              <Link href={`/a/${account}/campaigns/${c.id}`} className="font-heading text-xl leading-snug font-semibold hover:underline">
                {c.subject || "Untitled draft"}
              </Link>
              <CampaignStatusBadge status={c.status} />
            </div>
            <dl className="grid grid-cols-3 gap-3">
              {([["Readers", c.recipients], ["Opened", c.opened], ["Clicked", c.clicked]] as const).map(([label, n]) => (
                <div key={label} className="flex flex-col gap-1">
                  <dt className="label-caps text-muted-foreground">{label}</dt>
                  <dd className="tabular-nums text-xl font-semibold">{num.format(n)}</dd>
                </div>
              ))}
            </dl>
          </>
        ) : (
          <p className="text-muted-foreground">No issues yet. Write one, send yourself a test, then send it in safe waves.</p>
        )}
      </CardContent>
    </Card>
  );
}

function Stat({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <Card className="sketch-wobble">
      <CardContent className="flex flex-col gap-1.5">
        <span className="label-caps text-muted-foreground">{label}</span>
        <span className="tabular-nums text-3xl font-semibold tracking-tight">{value}</span>
        <span className="text-sm text-muted-foreground">{note}</span>
      </CardContent>
    </Card>
  );
}

/**
 * A rate against the limit that pauses sending: the bar fills toward the limit and turns amber past half, red at it.
 * Auto-pause ignores the rate until `minSent` emails went out, so before that the bar stays grey.
 */
function Rate({ label, rate, limit, sent, minSent }: { label: string; rate: number; limit: number; sent: number; minSent: number }) {
  const fill = limit > 0 ? rate / limit : 0;
  const judged = sent >= minSent;
  const tone = !judged ? "bg-muted-foreground/40" : fill >= 1 ? "bg-destructive" : fill >= 0.5 ? "bg-wait" : "bg-good";
  return (
    <div className="flex flex-col gap-2">
      <dt className="flex items-baseline justify-between gap-3">
        <span className="font-medium">{label}</span>
        <span className="tabular-nums text-sm">
          {pct(rate)} <span className="text-muted-foreground">/ {pct(limit)} limit</span>
        </span>
      </dt>
      <dd className="flex flex-col gap-2">
        <div
          role="meter"
          aria-label={`${label} against the pause limit`}
          aria-valuemin={0}
          aria-valuemax={limit}
          aria-valuenow={rate}
          aria-valuetext={`${pct(rate)} of a ${pct(limit)} limit`}
          className="relative h-1.5 rounded-full bg-foreground/10"
        >
          <div className={cn("h-full rounded-full", tone)} style={{ width: `${Math.min(fill, 1) * 100}%` }} />
          {/* The limit itself, drawn at the end of the track. */}
          <span aria-hidden className="absolute -top-1 right-0 h-3.5 w-0.5 rounded-full bg-destructive" />
        </div>
        <span className="text-sm text-muted-foreground">
          {judged ? `Sending pauses above ${pct(limit)}.` : `Not checked yet: ${num.format(sent)} of ${num.format(minSent)} emails sent. Sending pauses above ${pct(limit)} after that.`}
        </span>
      </dd>
    </div>
  );
}

async function Stats({ params }: Props) {
  const { account } = await params;
  const nextPath = `/a/${account}/overview`;
  const [o, latest] = await Promise.all([
    engine<Schemas["Overview"]>("/stats/overview", { account, nextPath }),
    engine<Schemas["CampaignList"]>("/campaigns?limit=1", { account, nextPath }),
  ]);
  const w = o.last_7d;
  const over = (r: number, l: number) => l > 0 && r >= l;
  const health: { tone: Tone; text: string } = o.status === "paused" ? { tone: "bad", text: "Paused" } : over(o.bounce_rate, o.bounce_limit / 2) || over(o.complaint_rate, o.complaint_limit / 2) ? { tone: "wait", text: "Watch" } : { tone: "good", text: "Healthy" };
  const rows: [string, keyof Schemas["Counts"]][] = [["Sent", "sent"], ["Delivered", "delivered"], ["Bounced", "bounced"], ["Complained", "complained"], ["Opened", "opened"], ["Clicked", "clicked"]];
  return (
    <>
      <PageHeader
        title="Overview"
        description="Your newsletter and your app email, last 7 days."
        actions={
          <form action={createCampaign.bind(null, account)}>
            <Button type="submit" size="lg">
              <PlusIcon aria-hidden />
              New issue
            </Button>
          </form>
        }
      />
      <section aria-label="Last 7 days" className="grid grid-cols-2 gap-4 md:gap-5 xl:grid-cols-4">
        <Stat label="Sent" value={num.format(w.sent)} note={`${num.format(o.last_24h.sent)} in the last 24 h`} />
        <Stat label="Delivered" value={share(w.delivered, w.sent)} note={`${num.format(w.delivered)} emails reached the inbox`} />
        <Stat label="Open rate" value={share(w.opened, w.delivered)} note={`${num.format(w.opened)} opens`} />
        <Stat label="Click rate" value={share(w.clicked, w.delivered)} note={`${num.format(w.clicked)} clicks`} />
      </section>
      <div className="grid gap-5 lg:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)]">
        <section aria-labelledby="health" className="contents">
        <Card>
          <CardHeader>
            <h2 id="health" className="text-lg leading-snug font-semibold">Sending health</h2>
            <CardAction>
              <Stamp tone={health.tone}>{health.text}</Stamp>
            </CardAction>
          </CardHeader>
          <CardContent>
            <dl className="flex flex-col gap-6">
              <Rate label="Bounce rate" rate={o.bounce_rate} limit={o.bounce_limit} sent={w.sent} minSent={o.bounce_min_sent} />
              <Rate label="Complaint rate" rate={o.complaint_rate} limit={o.complaint_limit} sent={w.sent} minSent={o.complaint_min_sent} />
            </dl>
          </CardContent>
        </Card>
        </section>
        <LatestIssue account={account} c={latest.data[0]} />
      </div>
      <section aria-labelledby="events" className="flex flex-col gap-3">
          <h2 id="events" className="text-lg font-semibold">Events</h2>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Event</TableHead>
                <TableHead className="text-right">Last 24 h</TableHead>
                <TableHead className="text-right">Last 7 days</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map(([label, k]) => (
                <TableRow key={k}>
                  <TableCell>{label}</TableCell>
                  <TableCell className="text-right tabular-nums">{num.format(o.last_24h[k])}</TableCell>
                  <TableCell className="text-right tabular-nums">{num.format(w[k])}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
      </section>
    </>
  );
}
