import Link from "next/link";
import { Suspense } from "react";
import { Badge } from "@/components/ui/badge";
import { engine, type Schemas } from "@/lib/engine";
import { AddDomain } from "./add-domain";

type Props = { params: Promise<{ account: string }> };

export function StatusBadge({ status }: { status: Schemas["DomainStatus"] }) {
  return <Badge variant={status === "verified" ? "default" : status === "failed" ? "destructive" : "secondary"}>{status}</Badge>;
}

export default function Domains(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Domains</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
        <List {...props} />
      </Suspense>
    </div>
  );
}

async function List({ params }: Props) {
  const { account } = await params;
  const domains = await engine<Schemas["DomainList"]>("/domains", { account, nextPath: `/a/${account}/domains` });
  return (
    <>
      <AddDomain account={account} />
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead><tr><th className="text-left">Domain</th><th className="text-left">Status</th><th className="text-left">Tracking</th><th className="text-right">Created</th></tr></thead>
          <tbody>
            {domains.data.map((d) => (
              <tr key={d.id}>
                <td><Link className="underline" href={`/a/${account}/domains/${d.id}`}>{d.name}</Link></td>
                <td><StatusBadge status={d.status} /></td>
                <td>{[d.open_tracking && "opens", d.click_tracking && "clicks"].filter(Boolean).join(", ") || "off"}</td>
                <td className="text-right tabular-nums">{new Date(d.created_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {domains.data.length === 0 && <p className="text-muted-foreground">No domains yet.</p>}
    </>
  );
}
