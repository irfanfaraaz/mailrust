import { Suspense } from "react";
import { cn } from "cn";
import { Loading } from "@/components/loading";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";

export const metadata = { title: "Overview" };

const pct = (n: number) => `${(n * 100).toFixed(2)}%`;
const num = new Intl.NumberFormat("en");

type Props = { params: Promise<{ account: string }> };

export default function Overview(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      <h1 className="text-2xl font-semibold">Overview</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading rows={5} />}>
        <Stats {...props} />
      </Suspense>
    </div>
  );
}

/**
 * A rate against the limit that pauses sending: the bar fills toward the limit and turns amber past half, red at it.
 * Auto-pause ignores the rate until `minSent` emails went out, so before that the bar stays grey.
 */
function Rate({ label, rate, limit, sent, minSent }: { label: string; rate: number; limit: number; sent: number; minSent: number }) {
  const share = limit > 0 ? rate / limit : 0;
  const judged = sent >= minSent;
  const tone = !judged ? "bg-muted-foreground/40" : share >= 1 ? "bg-destructive" : share >= 0.5 ? "bg-amber-500" : "bg-emerald-500";
  return (
    <div className="flex flex-col gap-2">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="flex flex-col gap-2">
        <span className="text-3xl font-semibold tabular-nums">{pct(rate)}</span>
        <div
          role="meter"
          aria-label={`${label} against the pause limit`}
          aria-valuemin={0}
          aria-valuemax={limit}
          aria-valuenow={rate}
          aria-valuetext={`${pct(rate)} of a ${pct(limit)} limit`}
          className="h-1.5 overflow-hidden rounded-full bg-muted"
        >
          <div className={cn("h-full rounded-full", tone)} style={{ width: `${Math.min(share, 1) * 100}%` }} />
        </div>
        <span className="text-sm text-muted-foreground">
          {judged ? `Sending pauses above ${pct(limit)}` : `Not checked yet: ${num.format(sent)} of ${num.format(minSent)} emails sent. Sending pauses above ${pct(limit)} after that.`}
        </span>
      </dd>
    </div>
  );
}

async function Stats({ params }: Props) {
  const { account } = await params;
  const o = await engine<Schemas["Overview"]>("/stats/overview", { account, nextPath: `/a/${account}/overview` });
  const rows: [string, keyof Schemas["Counts"]][] = [["Sent", "sent"], ["Delivered", "delivered"], ["Bounced", "bounced"], ["Complained", "complained"], ["Opened", "opened"], ["Clicked", "clicked"]];
  return (
    <>
      <section aria-labelledby="health" className="flex flex-col gap-4">
        <h2 id="health" className="text-lg font-medium">Sending health · last 7 days</h2>
        <dl className="grid max-w-3xl gap-8 sm:grid-cols-2">
          <Rate label="Bounce rate" rate={o.bounce_rate} limit={o.bounce_limit} sent={o.last_7d.sent} minSent={o.bounce_min_sent} />
          <Rate label="Complaint rate" rate={o.complaint_rate} limit={o.complaint_limit} sent={o.last_7d.sent} minSent={o.complaint_min_sent} />
        </dl>
      </section>
      <section aria-labelledby="events" className="flex flex-col gap-3">
        <h2 id="events" className="text-lg font-medium">Events</h2>
        <Table className="max-w-xl tabular-nums">
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
                <TableCell className="text-right">{num.format(o.last_24h[k])}</TableCell>
                <TableCell className="text-right">{num.format(o.last_7d[k])}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </section>
    </>
  );
}
