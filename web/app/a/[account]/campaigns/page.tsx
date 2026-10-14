import Link from "next/link";
import { Suspense } from "react";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { CampaignStatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { createCampaign } from "./actions";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Campaigns" };

type Props = { params: Promise<{ account: string }>; searchParams: Promise<{ after?: string }> };

export default function Campaigns(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      {/* params, searchParams and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
        <List {...props} />
      </Suspense>
    </div>
  );
}

async function List({ params, searchParams }: Props) {
  const { account } = await params;
  const { after } = await searchParams;
  const base = `/a/${account}/campaigns`;
  const list = await engine<Schemas["CampaignList"]>(`/campaigns?limit=50${after ? `&after=${after}` : ""}`, { account, nextPath: base });
  return (
    <>
      <PageHeader
        title="Campaigns"
        description="Every issue you have written, sent or scheduled."
        actions={
          <form action={createCampaign.bind(null, account)}>
            <Button type="submit" size="lg">New issue</Button>
          </form>
        }
      />
      {list.data.length === 0 && !after ? (
        <EmptyState title="No issues yet" hint="Write an issue in the editor, send yourself a test, then send it to your subscribers in safe waves." />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Subject</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="max-md:hidden text-right">Readers</TableHead>
              <TableHead className="max-md:hidden text-right">Opened</TableHead>
              <TableHead className="max-md:hidden text-right">Clicked</TableHead>
              <TableHead className="max-md:hidden text-right">Updated</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.data.map((c) => (
              <TableRow key={c.id}>
                <TableCell className="font-medium">
                  <Link className="text-primary underline-offset-2 hover:underline" href={`${base}/${c.id}`}>{c.subject || "Untitled draft"}</Link>
                </TableCell>
                <TableCell><CampaignStatusBadge status={c.status} /></TableCell>
                <TableCell className="max-md:hidden text-right tabular-nums">{c.recipients.toLocaleString("en-US")}</TableCell>
                <TableCell className="max-md:hidden text-right tabular-nums">{c.opened.toLocaleString("en-US")}</TableCell>
                <TableCell className="max-md:hidden text-right tabular-nums">{c.clicked.toLocaleString("en-US")}</TableCell>
                <TableCell className="max-md:hidden text-right tabular-nums text-muted-foreground"><LocalTime iso={c.updated_at} /></TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      {list.next_cursor && <Link className="self-start text-primary underline underline-offset-2" href={`?after=${list.next_cursor}`}>Older →</Link>}
    </>
  );
}
