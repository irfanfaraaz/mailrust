"use client";
import { Button } from "@/components/ui/button";

export default function AccountError({ error, reset }: { error: Error & { digest?: string }; reset: () => void }) {
  return (
    <div className="flex flex-col items-start gap-3">
      <h1 className="text-2xl font-semibold">Something went wrong</h1>
      {/* Engine messages are safe to show. */}
      <p>{error.message}</p>
      <Button onClick={reset}>Try again</Button>
    </div>
  );
}
