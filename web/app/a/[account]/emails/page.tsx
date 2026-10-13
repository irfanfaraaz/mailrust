import Link from "next/link";
import { Suspense } from "react";
import { engine, type Schemas } from "@/lib/engine";
import { Filters } from "./filters";

type SP = { status?: string; priority?: string; q?: string; since?: string; until?: string; after?: string };
type Props = { params: Promise<{ account: string }>; searchParams: Promise<SP> };

export default function Emails(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Emails</h1>
      {/* params, searchParams and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
        <Log {...props} />
      </Suspense>
    </div>
  );
}

async function Log({ params, searchParams }: Props) {
  const { account } = await params;
  const sp = await searchParams;
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(sp)) if (v) qs.set(k, k === "since" || k === "until" ? new Date(v).toISOString() : v);
  qs.set("limit", "50");
  const list = await engine<Schemas["EmailList"]>(`/emails?${qs}`, { account, nextPath: `/a/${account}/emails` });
  const older = new URLSearchParams({ ...sp, after: list.next_cursor ?? "" } as Record<string, string>);
  return (
    <>
      <Filters values={sp} />
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead><tr><th className="text-left">Subject</th><th className="text-left">To</th><th>Status</th><th>Priority</th><th className="text-right">Created</th></tr></thead>
          <tbody>
            {list.data.map((e) => (
              <tr key={e.id}>
                <td><Link className="underline-offset-2 hover:underline" href={`/a/${account}/emails/${e.id}`}>{e.subject}</Link></td>
                <td className="max-w-64 truncate">{e.to.join(", ")}</td>
                <td>{e.status}</td>
                <td>{e.priority}</td>
                <td className="text-right tabular-nums">{new Date(e.created_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {list.data.length === 0 && <p className="text-muted-foreground">No emails match.</p>}
      {list.next_cursor && <Link className="self-start underline" href={`?${older}`}>Older emails →</Link>}
    </>
  );
}
