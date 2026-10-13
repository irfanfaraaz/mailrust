import { cn } from "cn";

/** Placeholder bars shown while a Suspense boundary waits on the engine. */
export function Loading({ rows = 4, className }: { rows?: number; className?: string }) {
  return (
    <div role="status" aria-label="Loading" className={cn("flex flex-col gap-3", className)}>
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className="h-8 animate-pulse rounded-md bg-muted motion-reduce:animate-none" style={{ width: `${100 - i * 12}%` }} />
      ))}
      <span className="sr-only">Loading…</span>
    </div>
  );
}
