import { Suspense } from "react";
import { engine, type Schemas } from "@/lib/engine";

const pct = (n: number, digits = 2) => `${(n * 100).toFixed(digits)}%`;

type Props = { params: Promise<{ account: string }> };

export default function Overview(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      <h1 className="text-2xl font-semibold">Overview</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
        <Stats {...props} />
      </Suspense>
    </div>
  );
}

async function Stats({ params }: Props) {
  const { account } = await params;
  const o = await engine<Schemas["Overview"]>("/stats/overview", { account, nextPath: `/a/${account}/overview` });
  const rows: [string, keyof Schemas["Counts"]][] = [["Sent", "sent"], ["Delivered", "delivered"], ["Bounced", "bounced"], ["Complained", "complained"], ["Opened", "opened"], ["Clicked", "clicked"]];
  return (
    <>
      <table className="w-full max-w-xl text-sm tabular-nums">
        <thead><tr><th className="text-left">Event</th><th className="text-right">Last 24 h</th><th className="text-right">Last 7 days</th></tr></thead>
        <tbody>
          {rows.map(([label, k]) => (
            <tr key={k}><td>{label}</td><td className="text-right">{o.last_24h[k]}</td><td className="text-right">{o.last_7d[k]}</td></tr>
          ))}
        </tbody>
      </table>
      <section aria-label="Sending health" className="flex flex-col gap-2 text-sm">
        <p>Bounce rate (7 days): <strong>{pct(o.bounce_rate)}</strong> — sending pauses above {pct(o.bounce_limit, 0)}</p>
        <p>Complaint rate (7 days): <strong>{pct(o.complaint_rate, 3)}</strong> — sending pauses above {pct(o.complaint_limit)}</p>
      </section>
    </>
  );
}
