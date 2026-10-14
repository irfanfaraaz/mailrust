import { headers } from "next/headers";
import Link from "next/link";
import { Suspense } from "react";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { engine, type Schemas } from "@/lib/engine";
import { embedSnippet } from "@/lib/embed";
import { closedMessage } from "@/lib/format";
import { EmbedCode } from "./embed-code";
import { SettingsForm } from "./settings-form";

export const metadata = { title: "Signup form" };

type Props = { params: Promise<{ account: string }> };

export default function SignupForm(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title="Signup form"
        description="Readers join on a page Mailrust hosts, or through a form on your own site. Each one gets an email to confirm the address before they count as subscribed."
      />
      {/* params, headers and the engine calls are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
        <Settings {...props} />
      </Suspense>
    </div>
  );
}

async function Settings({ params }: Props) {
  const { account } = await params;
  const nextPath = `/a/${account}/signup-form`;
  const [s, domains] = await Promise.all([
    engine<Schemas["NewsletterSettings"]>("/newsletter", { account, nextPath }),
    engine<Schemas["DomainList"]>("/domains", { account, nextPath }),
  ]);
  const h = await headers();
  const origin = `${h.get("x-forwarded-proto") ?? "http"}://${h.get("x-forwarded-host") ?? h.get("host")}`;
  const verified = domains.data.filter((d) => d.status === "verified").map((d) => d.name);
  const link = s.public_slug ? `${origin}/join/${s.public_slug}` : null;
  const closed = s.signup_open ? null : closedMessage(s.closed_reason ?? "unknown reason");
  return (
    <>
      <p role="status" className={closed ? "max-w-prose text-sm text-muted-foreground" : "text-sm font-medium text-good"}>
        {closed ? closed.text : "Open: the form takes signups."}
        {closed?.domains && <> <Link className="text-primary underline underline-offset-2" href={`/a/${account}/domains`}>Go to Domains</Link></>}
      </p>
      <SettingsForm account={account} settings={s} domains={verified} origin={origin} />
      {link && (
        <section aria-labelledby="share-title">
          <Card className="max-w-xl">
            <CardHeader>
              <h2 id="share-title" className="text-lg leading-snug font-semibold">Share it</h2>
              <p className="text-sm text-muted-foreground">
                Public page (preview): <a className="break-all text-primary underline underline-offset-2" href={link} target="_blank" rel="noreferrer">{link}</a>
              </p>
            </CardHeader>
            <CardContent>
              <EmbedCode code={embedSnippet(origin, s.public_slug!, s.newsletter_name ?? "the newsletter")} />
            </CardContent>
          </Card>
        </section>
      )}
    </>
  );
}
