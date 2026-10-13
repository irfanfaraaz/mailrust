import { Suspense } from "react";
import { engine, type Schemas } from "@/lib/engine";
import { noJsUnsubState } from "@/lib/public-routes";
import { UnsubscribeForm } from "./unsubscribe-form";

export const metadata = { title: "Unsubscribe" };

type Props = { params: Promise<{ token: string }>; searchParams: Promise<{ done?: string }> };

export default function Unsubscribe(props: Props) {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-4 px-4">
      <h1 className="text-2xl font-semibold">Unsubscribe</h1>
      <Suspense>
        <Body {...props} />
      </Suspense>
    </main>
  );
}

async function Body({ params, searchParams }: Props) {
  const { token } = await params;
  const { done } = await searchParams;
  const info = await engine<Schemas["UnsubscribeView"]>(`/public/unsubscribe/${encodeURIComponent(token)}`, { auth: false });
  // An invalid link or a deleted contact: there is nothing to stop, and saying so would reveal nothing useful.
  if (!info.newsletter_name) return <p role="status" className="text-lg">You&apos;re unsubscribed.</p>;
  return <UnsubscribeForm token={token} name={info.newsletter_name} initial={noJsUnsubState(done)} />;
}
