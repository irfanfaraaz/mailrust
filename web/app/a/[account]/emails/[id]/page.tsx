import { Suspense } from "react";
import { ConfirmAction } from "@/components/confirm-action";
import { Loading } from "@/components/loading";
import { EmailStatusBadge, statusLabel } from "@/components/status-badge";
import { buttonVariants } from "@/components/ui/button";
import { engine, type Schemas } from "@/lib/engine";
import { cancelEmail } from "./actions";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Email" };

type Props = { params: Promise<{ account: string; id: string }> };

export default function EmailDetail(props: Props) {
  return (
    <Suspense fallback={<Loading rows={5} />}>
      <Detail {...props} />
    </Suspense>
  );
}

async function Detail({ params }: Props) {
  const { account, id } = await params;
  const e = await engine<Schemas["EmailView"]>(`/emails/${id}`, { account, nextPath: `/a/${account}/emails/${id}` });
  return (
    <article className="flex flex-col gap-6">
      <header className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold">{e.subject}</h1>
        <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <EmailStatusBadge status={e.status} />
          <span>From {e.from} to {e.to.join(", ")}</span>
          <span aria-hidden>·</span>
          <span>{statusLabel(e.priority)} priority</span>
        </p>
      </header>
      {(e.status === "queued" || e.status === "scheduled") && (
        <ConfirmAction
          trigger="Cancel this email"
          triggerClassName={`${buttonVariants({ variant: "outline" })} self-start`}
          title="Cancel this email?"
          description="It will not be sent. This cannot be undone."
          dismiss="Keep it"
          confirm="Cancel email"
          done="Email cancelled"
          action={cancelEmail.bind(null, account, id)}
        />
      )}
      {e.last_error && <p role="alert" className="text-sm text-destructive">{e.last_error}</p>}
      <section className="flex flex-col gap-2">
        <h2 className="text-lg font-medium">Events</h2>
        <ol aria-label="Events" className="flex flex-col gap-1 text-sm">
          {e.events.map((ev, i) => (
            <li key={i}>
              <span className="tabular-nums text-muted-foreground"><LocalTime iso={ev.created_at} /></span> — {ev.type}
              {ev.recipient ? ` (${ev.recipient})` : ""}
              {ev.likely_bot ? " · likely a bot" : ""}
            </li>
          ))}
          {e.events.length === 0 && <li className="text-muted-foreground">Nothing yet.</li>}
        </ol>
      </section>
      <section className="flex flex-col gap-2">
        <h2 className="text-lg font-medium">Body</h2>
        {e.html ? (
          // Customer HTML: never add allow-scripts. Tracking is added at send time, so viewing here counts no open.
          <iframe title="Email body" sandbox="allow-same-origin" srcDoc={e.html} height={560} className="w-full rounded-lg border bg-white" />
        ) : (
          <pre className="max-h-96 overflow-auto whitespace-pre-wrap rounded-lg border p-3 text-sm">{e.text ?? "(Hidden: login and invite emails hold one-time links.)"}</pre>
        )}
      </section>
    </article>
  );
}
