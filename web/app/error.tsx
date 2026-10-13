"use client";
import { useTransition } from "react";
import { Button } from "@/components/ui/button";

/** Catches engine failures on the public pages (join, confirm, unsubscribe); the account area has its own boundary. */
export default function PublicError({ error, retry }: { error: Error & { digest?: string }; retry: () => void }) {
  const [pending, start] = useTransition();
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-sm flex-col justify-center gap-4 px-4">
      <h1 className="text-2xl font-semibold">Something went wrong</h1>
      <p>This page could not load, usually because of a short outage. Try again in a moment.</p>
      <Button className="self-start" disabled={pending} onClick={() => start(() => retry())}>
        {pending ? "Retrying…" : "Try again"}
      </Button>
      {error.digest && <p className="text-xs text-muted-foreground">Reference: <code className="font-mono">{error.digest}</code></p>}
    </main>
  );
}
