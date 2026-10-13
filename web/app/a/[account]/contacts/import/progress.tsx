"use client";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { Schemas } from "@/lib/engine";

export function ImportProgress({ job }: { job: Schemas["ImportView"] }) {
  const router = useRouter();
  const running = job.status === "queued" || job.status === "running";
  useEffect(() => {
    if (!running) return;
    const t = setInterval(() => router.refresh(), 2000);
    return () => clearInterval(t);
  }, [running, router]);
  const title = running ? `Importing… ${job.rows_done.toLocaleString()} of ${job.rows_total.toLocaleString()} rows` : job.status === "done" ? "Import done" : "Import failed";
  return (
    <section aria-live="polite" className="flex flex-col gap-3 rounded-lg border p-4">
      <h2 className="text-lg font-medium">{title}</h2>
      <progress max={job.rows_total} value={job.rows_done} className="w-full" />
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
