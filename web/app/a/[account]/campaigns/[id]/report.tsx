import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { Schemas } from "@/lib/engine";
import { utc } from "@/lib/format";
import { pct } from "@/lib/report";
import { AutoRefresh } from "./auto-refresh";
import { Controls } from "./controls";

const fmt = (x: number) => x.toLocaleString("en-US");

type Props = { account: string; campaign: Schemas["Campaign"]; report: Schemas["CampaignReport"]; archiveUrl: string | null };

export function Report({ account, campaign: c, report: r, archiveUrl }: Props) {
  const t = r.totals;
  const tiles: [string, string, string?][] = [
    ["Sent", fmt(t.sent), "total-sent"],
    ["Delivered", fmt(t.delivered)],
    ["Opened", `${fmt(t.opened)} · ${pct(r.rates.open_rate)}`],
    ["Clicked", `${fmt(t.clicked)} · ${pct(r.rates.click_rate)}`],
    ["Bounced", `${fmt(t.bounced)} · ${pct(r.rates.bounce_rate)}`],
    ["Marked as spam", `${fmt(t.complained)} · ${pct(r.rates.complaint_rate)}`],
    ["Unsubscribed", `${fmt(t.unsubscribed)} · ${pct(r.rates.unsubscribe_rate)}`, "total-unsubscribed"],
    ["Skipped", fmt(t.skipped)],
  ];
  return (
    <section aria-labelledby="report-title" className="flex flex-col gap-6">
      <AutoRefresh active={c.status === "sending"} />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 id="report-title" className="text-lg font-medium">Report</h2>
        <Controls account={account} campaign={c} />
      </div>
      {c.paused_reason && <p role="status" className="text-sm text-amber-700 dark:text-amber-300">Paused: {c.paused_reason}</p>}
      <p className="text-sm text-muted-foreground">
        {c.started_at ? `Started ${utc(c.started_at)}` : "Not started"}
        {c.finished_at ? ` · finished ${utc(c.finished_at)}` : ""}
        {archiveUrl && <> · <a className="underline underline-offset-2" href={archiveUrl}>Public archive page</a></>}
      </p>
      <dl className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        {tiles.map(([label, value, testId]) => (
          <div key={label} className="rounded-lg border p-3">
            <dt className="text-sm text-muted-foreground">{label}</dt>
            <dd className="text-xl font-semibold tabular-nums" data-testid={testId}>{value}</dd>
          </div>
        ))}
      </dl>
      <p className="text-xs text-muted-foreground">Rates are over emails sent. Opens and clicks count each reader once; mail scanners are left out.</p>
      <div className="flex flex-col gap-2">
        <h3 className="font-medium">Waves</h3>
        <ul className="flex flex-col gap-1 text-sm">
          {r.waves.map((w) => <li key={w.wave} className="tabular-nums">Wave {w.wave}: {fmt(w.done)} of {fmt(w.readers)} done</li>)}
        </ul>
      </div>
      <div className="flex flex-col gap-2">
        <h3 className="font-medium">Clicks per link</h3>
        {r.links.length === 0 ? <p className="text-sm text-muted-foreground">No clicks yet.</p> : (
          <Table>
            <TableHeader><TableRow><TableHead>Link</TableHead><TableHead className="text-right">Clicks</TableHead><TableHead className="text-right">Readers</TableHead></TableRow></TableHeader>
            <TableBody>
              {r.links.map((l) => (
                <TableRow key={l.url}><TableCell className="max-w-xs truncate">{l.url}</TableCell><TableCell className="text-right tabular-nums">{fmt(l.clicks)}</TableCell><TableCell className="text-right tabular-nums">{fmt(l.unique_clicks)}</TableCell></TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </div>
      <div className="flex flex-col gap-2">
        <h3 className="font-medium">By mailbox provider</h3>
        <Table>
          <TableHeader><TableRow><TableHead>Provider</TableHead><TableHead className="text-right">Sent</TableHead><TableHead className="text-right">Opened</TableHead><TableHead className="text-right">Clicked</TableHead><TableHead className="text-right">Bounced</TableHead></TableRow></TableHeader>
          <TableBody>
            {r.providers.map((p) => (
              <TableRow key={p.provider}>
                <TableCell>{p.provider}</TableCell>
                <TableCell className="text-right tabular-nums">{fmt(p.sent)}</TableCell>
                <TableCell className="text-right tabular-nums">{fmt(p.opened)}</TableCell>
                <TableCell className="text-right tabular-nums">{fmt(p.clicked)}</TableCell>
                <TableCell className="text-right tabular-nums">{fmt(p.bounced)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </section>
  );
}
