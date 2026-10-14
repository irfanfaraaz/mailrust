import { notFound } from "next/navigation";
import { Suspense } from "react";
import { buttonVariants } from "@/components/ui/button";
import { engine, EngineError, type Schemas } from "@/lib/engine";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Archive" };

type Props = { params: Promise<{ slug: string }> };


export default function Archive(props: Props) {
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-[65ch] flex-col gap-8 px-4 py-12">
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
        <Issues {...props} />
      </Suspense>
    </main>
  );
}

async function Issues({ params }: Props) {
  const { slug } = await params;
  let a: Schemas["ArchiveIndex"];
  try {
    a = await engine<Schemas["ArchiveIndex"]>(`/public/archive/${encodeURIComponent(slug)}`, { auth: false });
  } catch (e) {
    if (e instanceof EngineError && e.status === 404) notFound();
    throw e;
  }
  return (
    <>
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-dashed pb-5">
        <h1 className="text-3xl font-semibold">{a.newsletter_name}</h1>
        {a.signup_open && <a className={buttonVariants({ size: "sm" })} href={`/join/${encodeURIComponent(slug)}`}>Subscribe</a>}
      </header>
      {a.issues.length === 0 ? (
        <p className="text-muted-foreground">No issues yet.</p>
      ) : (
        <ul className="flex flex-col divide-y divide-dashed">
          {a.issues.map((i) => (
            <li key={i.slug} className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 py-3.5">
              {/* A plain link: the issue is a route handler page, not a React page. */}
              <a className="font-heading text-lg font-medium text-primary underline-offset-2 hover:underline" href={`/p/${encodeURIComponent(slug)}/${encodeURIComponent(i.slug)}`}>{i.title}</a>
              <LocalTime iso={i.sent_at} dateOnly className="font-mono text-sm text-muted-foreground tabular-nums" />
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
