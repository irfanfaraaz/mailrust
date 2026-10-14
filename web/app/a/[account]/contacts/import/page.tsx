import Link from "next/link";
import { Suspense } from "react";
import { BackLink } from "@/components/back-link";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { ImportStatusBadge } from "@/components/status-badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { ImportProgress } from "./progress";
import { UploadForm } from "./upload-form";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Import contacts" };

type Props = { params: Promise<{ account: string }>; searchParams: Promise<{ job?: string }> };

export default function Import(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader title="Import contacts" description="Bring people over from a CSV file.">
        {/* The back link needs the account from params: request data, so it reads inside Suspense too. */}
        <Suspense fallback={<span className="h-5" />}>
          <Back params={props.params} />
        </Suspense>
      </PageHeader>
      <Suspense fallback={<Loading />}>
        <Body {...props} />
      </Suspense>
    </div>
  );
}

async function Back({ params }: Pick<Props, "params">) {
  const { account } = await params;
  return <BackLink href={`/a/${account}/contacts`}>Contacts</BackLink>;
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
        <section className="flex flex-col gap-3">
          <h2 className="text-lg font-semibold">Past imports</h2>
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
                  <TableCell className="tabular-nums"><LocalTime iso={p.created_at} /></TableCell>
                  <TableCell><ImportStatusBadge status={p.status} /></TableCell>
                  <TableCell className="text-right tabular-nums">{p.rows_total}</TableCell>
                  <TableCell className="text-right tabular-nums">{p.added}</TableCell>
                  <TableCell className="text-right tabular-nums">{p.updated}</TableCell>
                  <TableCell className="text-right tabular-nums">{p.skipped}</TableCell>
                  <TableCell className="text-right"><Link className="text-primary underline underline-offset-2" href={`?job=${p.id}`}>Report</Link></TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </section>
      )}
    </>
  );
}
