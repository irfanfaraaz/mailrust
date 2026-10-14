import Link from "next/link";
import { Suspense } from "react";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { EmailStatusBadge, statusLabel } from "@/components/status-badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { isoDate } from "@/lib/format";
import { Filters } from "./filters";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Emails" };

type SP = { status?: string; priority?: string; q?: string; since?: string; until?: string; after?: string };
type Props = { params: Promise<{ account: string }>; searchParams: Promise<SP> };

export default function Emails(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader title="Emails" description="Every email your app sent through the API, newest first." />
      {/* params, searchParams and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading rows={6} />}>
        <Log {...props} />
      </Suspense>
    </div>
  );
}

async function Log({ params, searchParams }: Props) {
  const { account } = await params;
  const sp = await searchParams;
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(sp)) {
    const value = v && (k === "since" || k === "until" ? isoDate(v) : v);
    if (value) qs.set(k, value);
  }
  qs.set("limit", "50");
  const list = await engine<Schemas["EmailList"]>(`/emails?${qs}`, { account, nextPath: `/a/${account}/emails` });
  const older = new URLSearchParams({ ...sp, after: list.next_cursor ?? "" } as Record<string, string>);
  return (
    <>
      <Filters values={sp} />
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Subject</TableHead>
            <TableHead>To</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Priority</TableHead>
            <TableHead className="text-right">Created</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {list.data.map((e) => (
            <TableRow key={e.id}>
              <TableCell className="max-w-80 truncate">
                <Link className="font-medium text-primary underline-offset-2 hover:underline" href={`/a/${account}/emails/${e.id}`}>{e.subject}</Link>
              </TableCell>
              <TableCell className="max-w-64 truncate font-mono text-[0.8125rem]">{e.to.join(", ")}</TableCell>
              <TableCell><EmailStatusBadge status={e.status} /></TableCell>
              <TableCell className="text-muted-foreground">{statusLabel(e.priority)}</TableCell>
              <TableCell className="text-right tabular-nums text-muted-foreground"><LocalTime iso={e.created_at} /></TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {list.data.length === 0 && <EmptyState title="No emails match" hint="Change or clear the filters above to see more of the log." />}
      {list.next_cursor && <Link className="self-start font-medium text-primary underline underline-offset-2" href={`?${older}`}>Older emails →</Link>}
    </>
  );
}
