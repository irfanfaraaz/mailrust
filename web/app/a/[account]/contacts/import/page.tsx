import Link from "next/link";
import { Suspense } from "react";
import { Loading } from "@/components/loading";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { utc } from "@/lib/format";
import { ImportProgress } from "./progress";
import { UploadForm } from "./upload-form";

export const metadata = { title: "Import contacts" };

type Props = { params: Promise<{ account: string }>; searchParams: Promise<{ job?: string }> };

export default function Import(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <a href="../contacts" className="text-sm text-muted-foreground underline-offset-2 hover:underline">← Contacts</a>
      <h1 className="text-2xl font-semibold">Import contacts</h1>
      <Suspense fallback={<Loading />}>
        <Body {...props} />
      </Suspense>
    </div>
  );
}

async function Body({ params, searchParams }: Props) {
  const { account } = await params;
  const { job } = await searchParams;
  const nextPath = `/a/${account}/contacts/import`;
  const [tags, past, current] = await Promise.all([
    engine<Schemas["TagList"]>("/tags", { account, nextPath }),
    engine<Schemas["ImportList"]>("/contacts/imports", { account, nextPath }),
    job ? engine<Schemas["ImportView"]>(`/contacts/imports/${encodeURIComponent(job)}`, { account, nextPath }) : null,
  ]);
  return (
    <>
      {current && <ImportProgress job={current} />}
      <UploadForm account={account} tags={tags.data} />
      {past.data.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="text-lg font-medium">Past imports</h2>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Started</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Rows</TableHead>
                <TableHead className="text-right">Added</TableHead>
                <TableHead className="text-right">Updated</TableHead>
                <TableHead className="text-right">Skipped</TableHead>
                <TableHead><span className="sr-only">Report</span></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {past.data.map((p) => (
                <TableRow key={p.id}>
                  <TableCell className="tabular-nums">{utc(p.created_at)}</TableCell>
                  <TableCell>{p.status}</TableCell>
                  <TableCell className="text-right tabular-nums">{p.rows_total}</TableCell>
                  <TableCell className="text-right tabular-nums">{p.added}</TableCell>
                  <TableCell className="text-right tabular-nums">{p.updated}</TableCell>
                  <TableCell className="text-right tabular-nums">{p.skipped}</TableCell>
                  <TableCell className="text-right"><Link className="underline underline-offset-2" href={`?job=${p.id}`}>Report</Link></TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </section>
      )}
    </>
  );
}
