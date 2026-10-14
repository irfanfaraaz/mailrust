import { Suspense } from "react";
import { ConfirmAction } from "@/components/confirm-action";
import { BackLink } from "@/components/back-link";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { EmailStatusBadge, statusLabel } from "@/components/status-badge";
import { buttonVariants } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <dt className="label-caps text-muted-foreground">{label}</dt>
      <dd className="break-words font-mono text-[0.8125rem]">{children}</dd>
    </div>
  );
}

async function Detail({ params }: Props) {
  const { account, id } = await params;
  const e = await engine<Schemas["EmailView"]>(`/emails/${id}`, { account, nextPath: `/a/${account}/emails/${id}` });
  return (
    <article className="flex flex-col gap-8">
      <PageHeader
        title={<span className="break-words">{e.subject}</span>}
        description={`${statusLabel(e.priority)} priority`}
        actions={
          (e.status === "queued" || e.status === "scheduled") && (
            <ConfirmAction
              trigger="Cancel this email"
              triggerClassName={buttonVariants({ variant: "outline" })}
              title="Cancel this email?"
              description="It will not be sent. This cannot be undone."
              dismiss="Keep it"
              confirm="Cancel email"
              done="Email cancelled"
              action={cancelEmail.bind(null, account, id)}
            />
          )
        }
      >
        <BackLink href={`/a/${account}/emails`}>Emails</BackLink>
      </PageHeader>
      <Card>
        <CardContent>
          <dl className="grid gap-x-6 gap-y-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <dt className="label-caps text-muted-foreground">Status</dt>
              <dd><EmailStatusBadge status={e.status} /></dd>
            </div>
            <Field label="Id">{id}</Field>
            <Field label="From">{e.from}</Field>
            <Field label="To">{e.to.join(", ")}</Field>
          </dl>
          {e.last_error && <p role="alert" className="mt-4 font-mono text-[0.8125rem] text-destructive">{e.last_error}</p>}
        </CardContent>
      </Card>
      <section aria-labelledby="events" className="flex flex-col gap-3">
        <h2 id="events" className="text-lg font-semibold">Events</h2>
        <Card>
          <CardContent>
            <ol aria-label="Events" className="flex flex-col gap-2.5 text-sm">
              {e.events.map((ev, i) => (
                <li key={i} className="flex flex-wrap items-baseline gap-x-3 gap-y-0.5 border-l-2 border-foreground/25 pl-3">
                  <span className="tabular-nums text-muted-foreground"><LocalTime iso={ev.created_at} /></span>
                  <span className="sr-only">—</span>
                  <span className="font-mono text-[0.8125rem] font-medium">{ev.type}</span>
                  {ev.recipient ? <span className="font-mono text-[0.8125rem] text-muted-foreground">({ev.recipient})</span> : null}
                  {ev.likely_bot ? <span className="text-muted-foreground">· likely a bot</span> : null}
                </li>
              ))}
              {e.events.length === 0 && <li className="text-muted-foreground">Nothing yet.</li>}
            </ol>
          </CardContent>
        </Card>
      </section>
      <section aria-labelledby="body" className="flex flex-col gap-3">
        <h2 id="body" className="text-lg font-semibold">Body</h2>
        {e.html ? (
          // Customer HTML: never add allow-scripts. Tracking is added at send time, so viewing here counts no open.
          <iframe title="Email body" sandbox="allow-same-origin" srcDoc={e.html} height={560} className="sketch sketch-lg w-full bg-white" />
        ) : (
          <pre className="sketch sketch-lg max-h-96 overflow-auto bg-card p-4 font-mono text-[0.8125rem] whitespace-pre-wrap">{e.text ?? "(Hidden: login and invite emails hold one-time links.)"}</pre>
        )}
      </section>
    </article>
  );
}
