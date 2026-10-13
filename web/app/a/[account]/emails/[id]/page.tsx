import { Suspense } from "react";
import { engine, type Schemas } from "@/lib/engine";
import { cancelEmail } from "./actions";

type Props = { params: Promise<{ account: string; id: string }> };

export default function EmailDetail(props: Props) {
  return (
    <Suspense>
      <Detail {...props} />
    </Suspense>
  );
}

async function Detail({ params }: Props) {
  const { account, id } = await params;
  const e = await engine<Schemas["EmailView"]>(`/emails/${id}`, { account, nextPath: `/a/${account}/emails/${id}` });
  return (
    <article className="flex flex-col gap-6">
      <header className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold">{e.subject}</h1>
        <p className="text-sm text-muted-foreground">From {e.from} to {e.to.join(", ")} · {e.status} · {e.priority}</p>
      </header>
      {(e.status === "queued" || e.status === "scheduled") && (
        <form action={cancelEmail.bind(null, account, id)}><button className="underline" type="submit">Cancel this email</button></form>
      )}
      {e.last_error && <p role="alert" className="text-sm text-destructive">{e.last_error}</p>}
      <section>
        <h2 className="font-medium">Events</h2>
        <ol aria-label="Events" className="mt-2 flex flex-col gap-1 text-sm">
          {e.events.map((ev, i) => (
            <li key={i} className="tabular-nums">{new Date(ev.created_at).toLocaleString()} — {ev.type}{ev.recipient ? ` (${ev.recipient})` : ""}{ev.likely_bot ? " · likely a bot" : ""}</li>
          ))}
          {e.events.length === 0 && <li className="text-muted-foreground">Nothing yet.</li>}
        </ol>
      </section>
      <section>
        <h2 className="font-medium">Body</h2>
        {/* Customer content: show text only, never render e.html. */}
        <pre className="mt-2 max-h-96 overflow-auto whitespace-pre-wrap rounded border p-3 text-sm">{e.text ?? "(HTML only)"}</pre>
      </section>
    </article>
  );
}
