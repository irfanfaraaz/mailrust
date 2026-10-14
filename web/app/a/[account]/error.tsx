"use client";
import { useRouter } from "next/navigation";
import { useTransition } from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export default function AccountError({ error, reset }: { error: Error & { digest?: string }; reset: () => void }) {
  const router = useRouter();
  const [pending, start] = useTransition();
  return (
    <Card className="max-w-prose items-start gap-3 px-6 py-7">
      <h1 className="text-2xl font-semibold">Something went wrong</h1>
      {/* In production error.message is a generic string, so name the likely cause and give the digest for the server logs. */}
      <p>This page could not load. It is usually a short outage of the Mailrust engine. Try again in a moment.</p>
      {error.digest && (
        <p className="text-sm text-muted-foreground">
          If it keeps failing, search the server logs for <code className="font-mono text-foreground">{error.digest}</code>.
        </p>
      )}
      {/* Refresh refetches the server components; reset re-renders this segment. */}
      <Button disabled={pending} onClick={() => start(() => { router.refresh(); reset(); })}>
        {pending ? "Retrying…" : "Try again"}
      </Button>
    </Card>
  );
}
