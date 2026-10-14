"use client";
import { useState } from "react";
import { CopiedHint } from "@/components/copied-hint";
import { Button } from "@/components/ui/button";

export function CopyButton({ value, label }: { value: string; label: string }) {
  const [copied, setCopied] = useState(0);
  return (
    <span className="relative inline-flex">
      <Button
        type="button"
        variant="outline"
        size="sm"
        aria-label={`Copy ${label}`}
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(value);
            setCopied((n) => n + 1);
          } catch {
            /* clipboard unavailable: the value is visible to select by hand */
          }
        }}
      >
        {copied ? "Copied" : "Copy"}
      </Button>
      <CopiedHint n={copied} />
    </span>
  );
}
