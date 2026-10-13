import { cookies } from "next/headers";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { engine, EngineError, type Schemas } from "@/lib/engine";
import { DRAFT_COOKIE, readDraft } from "@/lib/join-draft";

export const metadata = { title: "Subscribe" };

type Props = { params: Promise<{ slug: string }>; searchParams: Promise<{ error?: string }> };

export default function Join(props: Props) {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-6 px-4">
      {/* params, searchParams and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
        <JoinForm {...props} />
      </Suspense>
    </main>
  );
}

async function JoinForm({ params, searchParams }: Props) {
  const { slug } = await params;
  const { error } = await searchParams;
  let n: Schemas["PublicNewsletter"];
  try {
    n = await engine<Schemas["PublicNewsletter"]>(`/public/newsletter/${encodeURIComponent(slug)}`, { auth: false });
  } catch (e) {
    if (e instanceof EngineError && e.status === 404) notFound();
    throw e;
  }
  if (!n.open) {
    return (
      <>
        <h1 className="text-2xl font-semibold">{n.name}</h1>
        <p>This newsletter is not taking signups right now.</p>
        <p className="text-sm text-muted-foreground">Check back later.</p>
      </>
    );
  }
  const bad = error === "email";
  const draft = bad ? readDraft((await cookies()).get(DRAFT_COOKIE)?.value) : { email: "", first_name: "" };
  return (
    <>
      <h1 className="text-2xl font-semibold">Subscribe to {n.name}</h1>
      {/* A plain POST, not a server action: the same route handler takes forms embedded on other sites. */}
      <form method="post" action={`/join/${encodeURIComponent(slug)}`} className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="email">Email</Label>
          <Input
            id="email"
            name="email"
            type="email"
            autoComplete="email"
            required
            defaultValue={draft.email}
            aria-invalid={bad || undefined}
            aria-describedby={bad ? "email-error" : undefined}
          />
          {bad && <p id="email-error" role="alert" className="text-sm text-destructive">That does not look like an email address. Check it and try again.</p>}
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="first_name">First name (optional)</Label>
          <Input id="first_name" name="first_name" autoComplete="given-name" defaultValue={draft.first_name} />
        </div>
        {/* Honeypot: hidden from people and screen readers; bots fill it in. */}
        <input type="text" name="website" tabIndex={-1} autoComplete="off" aria-hidden="true" className="absolute -left-[9999px] size-px" />
        <Button type="submit">Subscribe</Button>
      </form>
      <p className="text-xs text-muted-foreground">You will get an email to confirm. Unsubscribe any time from any issue.</p>
    </>
  );
}
