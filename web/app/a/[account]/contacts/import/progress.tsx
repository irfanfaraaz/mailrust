"use client";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
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
    <section aria-labelledby="import-title" className="flex flex-col gap-3 rounded-lg border p-4">
      {/* Only the status is announced, so screen readers hear each change once instead of every 2 s refresh. */}
      <p role="status" className="sr-only">{announce[job.status]}</p>
      <h2 id="import-title" className="text-lg font-medium">{title}</h2>
      <progress
        aria-label="Rows processed"
        max={Math.max(job.rows_total, 1)}
        value={job.status === "queued" ? undefined : job.rows_done}
        className="w-full"
      />
      <div className="flex flex-wrap gap-6 text-sm">
        <p>Added <strong className="tabular-nums">{job.added}</strong></p>
        <p>Updated <strong className="tabular-nums">{job.updated}</strong></p>
        <p>Skipped <strong className="tabular-nums">{job.skipped}</strong></p>
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
    </section>
  );
}
