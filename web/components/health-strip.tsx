import Link from "next/link";
import { cn } from "cn";
import { engine, type Schemas } from "@/lib/engine";

const pct = (n: number) => `${(n * 100).toFixed(1)}%`;

/** Sending health, pinned to the sidebar foot so it is never out of sight: bounce rate against the pause limit. */
export async function HealthStrip({ accountId }: { accountId: string }) {
  const o = await engine<Schemas["Overview"]>("/stats/overview", { account: accountId, nextPath: `/a/${accountId}/overview` });
  const paused = o.status === "paused";
  const share = o.bounce_limit > 0 ? Math.min(o.bounce_rate / o.bounce_limit, 1) : 0;
  const tone = paused || share >= 1 ? "bg-destructive" : share >= 0.5 ? "bg-wait" : "bg-good";
  return (
    <Link href={`/a/${accountId}/overview`} className="m-3 flex flex-col gap-2 rounded-md bg-sidebar-accent/70 p-3 text-sm hover:bg-sidebar-accent">
      <span className="flex items-baseline justify-between gap-2">
        <span className={cn("font-medium", paused && "text-destructive")}>{paused ? "Sending paused" : share >= 0.5 ? "Watch bounces" : "Sending healthy"}</span>
        <span className="tabular-nums text-xs text-muted-foreground">{pct(o.bounce_rate)} bounce</span>
      </span>
      <span aria-hidden className="h-1.5 overflow-hidden rounded-full bg-foreground/10">
        <span className={cn("block h-full rounded-full", tone)} style={{ width: `${Math.max(share * 100, 2)}%` }} />
      </span>
      <span className="flex justify-between text-xs text-muted-foreground">
        <span>Last 7 days</span>
        <span className="tabular-nums">{o.last_7d.sent.toLocaleString("en-US")} sent</span>
      </span>
    </Link>
  );
}
