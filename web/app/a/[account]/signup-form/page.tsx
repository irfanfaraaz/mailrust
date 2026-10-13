import { headers } from "next/headers";
import { Suspense } from "react";
import { Loading } from "@/components/loading";
import { engine, type Schemas } from "@/lib/engine";
import { embedSnippet } from "@/lib/embed";
import { EmbedCode } from "./embed-code";
import { SettingsForm } from "./settings-form";

export const metadata = { title: "Signup form" };

type Props = { params: Promise<{ account: string }> };

export default function SignupForm(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Signup form</h1>
      <p className="max-w-prose text-sm text-muted-foreground">
        Readers join on a page Mailrust hosts, or through a form on your own site. Each one gets an email to confirm the address before they count as subscribed.
      </p>
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
  return (
    <>
      <p role="status" className={s.signup_open ? "text-sm font-medium text-emerald-700 dark:text-emerald-300" : "text-sm text-muted-foreground"}>
        {s.signup_open ? "Open: the form takes signups." : `Closed: ${s.closed_reason}.`}
      </p>
      <SettingsForm account={account} settings={s} domains={verified} origin={origin} />
      {link && (
        <section className="flex max-w-xl flex-col gap-3">
          <h2 className="text-lg font-medium">Share it</h2>
          <p className="text-sm">
            Public page (preview): <a className="underline underline-offset-2" href={link} target="_blank" rel="noreferrer">{link}</a>
          </p>
          <EmbedCode code={embedSnippet(origin, s.public_slug!, s.newsletter_name ?? "the newsletter")} />
        </section>
      )}
    </>
  );
}
