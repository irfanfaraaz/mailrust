import Link from "next/link";
import { Suspense } from "react";
import { engine, type Schemas } from "@/lib/engine";

export const metadata = { title: "Check your email" };

type Props = { params: Promise<{ slug: string }> };

export default function CheckEmail({ params }: Props) {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-3 px-4">
      <h1 className="text-2xl font-semibold">Check your email</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<p>We sent you a link to confirm your subscription. It works for 7 days.</p>}>
        <Body params={params} />
      </Suspense>
    </main>
  );
}

async function Body({ params }: Props) {
  const { slug } = await params;
  const page = `/join/${encodeURIComponent(slug)}`;
  // Only the newsletter's public name: this page must read the same for every address, so it never says whether the email was new.
  const name = await engine<Schemas["PublicNewsletter"]>(`/public/newsletter/${encodeURIComponent(slug)}`, { auth: false })
    .then((n) => n.name, () => null);
  return (
    <>
      <p>We sent you a link to confirm your subscription{name ? <> to {name}</> : null}. It works for 7 days.</p>
      <p className="text-sm text-muted-foreground">
        Nothing there? Check your spam folder, or{" "}
        <Link href={page} className="text-foreground underline underline-offset-2">try a different address</Link>.
      </p>
    </>
  );
}
