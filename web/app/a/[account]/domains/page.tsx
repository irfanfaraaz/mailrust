import Link from "next/link";
import { Suspense } from "react";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { DomainStatusBadge } from "@/components/status-badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { utc } from "@/lib/format";
import { AddDomain } from "./add-domain";

export const metadata = { title: "Domains" };

type Props = { params: Promise<{ account: string }> };

export default function Domains(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Domains</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
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
      {domains.data.length === 0 ? (
        <EmptyState
          title="No domains yet"
          hint="Mailrust only sends from domains you have verified. Add one, then create the DNS records it shows you."
          action="Add your first domain"
          href="#domain-name"
        />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Domain</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Tracking</TableHead>
              <TableHead className="text-right">Created</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {domains.data.map((d) => (
              <TableRow key={d.id}>
                <TableCell><Link className="font-medium underline underline-offset-2" href={`/a/${account}/domains/${d.id}`}>{d.name}</Link></TableCell>
                <TableCell><DomainStatusBadge status={d.status} /></TableCell>
                <TableCell className="text-muted-foreground">{[d.open_tracking && "opens", d.click_tracking && "clicks"].filter(Boolean).join(", ") || "off"}</TableCell>
                <TableCell className="text-right tabular-nums text-muted-foreground">{utc(d.created_at)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </>
  );
}
