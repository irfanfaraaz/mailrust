import Link from "next/link";
import { Suspense } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { engine, type Schemas } from "@/lib/engine";
import { AddSuppression } from "./add-suppression";
import { removeSuppression } from "./actions";

type SP = { q?: string; after?: string };
type Props = { params: Promise<{ account: string }>; searchParams: Promise<SP> };

export default function Suppressions(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Suppressions</h1>
      <p className="text-sm text-muted-foreground">Mailrust never sends to these addresses. Bounces and complaints add them automatically.</p>
      {/* params, searchParams and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
        <List {...props} />
      </Suspense>
    </div>
  );
}

async function List({ params, searchParams }: Props) {
  const { account } = await params;
  const sp = await searchParams;
  const qs = new URLSearchParams({ limit: "50" });
  if (sp.q) qs.set("q", sp.q);
  if (sp.after) qs.set("after", sp.after);
  const list = await engine<Schemas["SuppressionList"]>(`/suppressions?${qs}`, { account, nextPath: `/a/${account}/suppressions` });
  const older = new URLSearchParams({ ...(sp.q ? { q: sp.q } : {}), after: list.next_cursor ?? "" });
  return (
    <>
      <AddSuppression account={account} />
      {/* A plain GET form: the search lives in the URL and needs no JS. */}
      <form method="get" className="flex flex-wrap items-end gap-3 text-sm">
        <label className="flex flex-col gap-1">Search
          <Input name="q" type="search" placeholder="Address" defaultValue={sp.q ?? ""} />
        </label>
        <Button type="submit" variant="secondary">Search</Button>
      </form>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead><tr><th className="text-left">Address</th><th className="text-left">Reason</th><th className="text-right">Added</th><th /></tr></thead>
          <tbody>
            {list.data.map((s) => (
              <tr key={s.address}>
                <td>{s.address}</td>
                <td>{s.reason}</td>
                <td className="text-right tabular-nums">{new Date(s.created_at).toLocaleString()}</td>
                <td className="text-right">
                  <form action={removeSuppression.bind(null, account, s.address)}>
                    <Button type="submit" variant="outline" size="sm">Unblock</Button>
                  </form>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {list.data.length === 0 && <p className="text-muted-foreground">No suppressed addresses.</p>}
      {list.next_cursor && <Link className="self-start underline" href={`?${older}`}>Older →</Link>}
    </>
  );
}
