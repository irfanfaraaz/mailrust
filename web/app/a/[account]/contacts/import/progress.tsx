"use client";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { Schemas } from "@/lib/engine";

const announce: Record<Schemas["ImportStatus"], string> = {
  queued: "Import queued",
  running: "Import running",
  done: "Import done",
  failed: "Import failed",
};

export function ImportProgress({ job }: { job: Schemas["ImportView"] }) {
  const router = useRouter();
  const running = job.status === "queued" || job.status === "running";
  useEffect(() => {
    if (!running) return;
    const t = setInterval(() => router.refresh(), 2000);
    return () => clearInterval(t);
  }, [running, router]);
  const title = {
    queued: "Import queued: waiting to start",
    running: `Importing… ${job.rows_done.toLocaleString()} of ${job.rows_total.toLocaleString()} rows`,
    done: "Import done",
    failed: "Import failed",
  }[job.status];
  return (
    <section aria-labelledby="import-title">
    <Card>
      <CardHeader>
        {/* Only the status is announced, so screen readers hear each change once instead of every 2 s refresh. */}
        <p role="status" className="sr-only">{announce[job.status]}</p>
        <h2 id="import-title" className="text-lg leading-snug font-semibold">{title}</h2>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
      <progress
        aria-label="Rows processed"
        max={Math.max(job.rows_total, 1)}
        value={job.status === "queued" ? undefined : job.rows_done}
        className="w-full accent-primary"
      />
      <div className="flex flex-wrap gap-8 text-sm">
        <p className="flex flex-col gap-1"><span className="label-caps text-muted-foreground">Added</span><strong className="tabular-nums text-2xl font-semibold">{job.added}</strong></p>
        <p className="flex flex-col gap-1"><span className="label-caps text-muted-foreground">Updated</span><strong className="tabular-nums text-2xl font-semibold">{job.updated}</strong></p>
        <p className="flex flex-col gap-1"><span className="label-caps text-muted-foreground">Skipped</span><strong className="tabular-nums text-2xl font-semibold">{job.skipped}</strong></p>
      </div>
      {job.error && <p role="alert" className="text-sm text-destructive">{job.error}</p>}
      {job.errors.length > 0 && (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-20">Line</TableHead>
              <TableHead>Problem</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {job.errors.map((e) => (
              <TableRow key={e.line}>
                <TableCell className="tabular-nums">{e.line}</TableCell>
                <TableCell>{e.reason}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      {job.skipped > job.errors.length && <p className="text-sm text-muted-foreground">Only the first 1,000 problems are listed.</p>}
      </CardContent>
    </Card>
    </section>
  );
}
