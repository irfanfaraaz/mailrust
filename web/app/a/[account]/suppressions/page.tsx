import Link from "next/link";
import { Suspense } from "react";
import { ConfirmAction } from "@/components/confirm-action";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { AddSuppression } from "./add-suppression";
import { removeSuppression } from "./actions";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Suppressions" };

type SP = { q?: string; after?: string };
type Props = { params: Promise<{ account: string }>; searchParams: Promise<SP> };

export default function Suppressions(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader title="Suppressions" description="Mailrust never sends to these addresses. Bounces and complaints add them automatically." />
      {/* params, searchParams and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
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
      <Card size="sm">
        <CardContent>
          <form method="get" className="flex flex-wrap items-end gap-3 text-sm">
            <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Search</span>
              <Input name="q" type="search" placeholder="Address" defaultValue={sp.q ?? ""} />
            </label>
            <Button type="submit" variant="outline">Search</Button>
          </form>
        </CardContent>
      </Card>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Address</TableHead>
            <TableHead>Reason</TableHead>
            <TableHead className="text-right">Added</TableHead>
            <TableHead><span className="sr-only">Actions</span></TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {list.data.map((s) => (
            <TableRow key={s.address}>
              <TableCell className="font-mono text-[0.8125rem]">{s.address}</TableCell>
              <TableCell className="text-muted-foreground">{s.reason}</TableCell>
              <TableCell className="text-right tabular-nums text-muted-foreground"><LocalTime iso={s.created_at} /></TableCell>
              <TableCell className="text-right">
                <ConfirmAction
                  trigger="Unblock"
                  title={`Unblock ${s.address}?`}
                  description="Mailrust will send to this address again. If it bounced or complained before, sending to it can raise your bounce or complaint rate."
                  confirm="Unblock address"
                  done="Address unblocked"
                  action={removeSuppression.bind(null, account, s.address)}
                />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {list.data.length === 0 && <EmptyState title="No suppressed addresses" hint="Addresses that bounce or complain are blocked here automatically." />}
      {list.next_cursor && <Link className="self-start font-medium text-primary underline underline-offset-2" href={`?${older}`}>Older →</Link>}
    </>
  );
}
