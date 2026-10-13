"use client";
import { useState } from "react";
import { Button } from "@/components/ui/button";

export function EmbedCode({ code }: { code: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="flex flex-col gap-2">
      <label className="flex flex-col gap-1 text-sm">Embed code
        <textarea readOnly rows={7} value={code} onFocus={(e) => e.currentTarget.select()} className="rounded-lg border bg-muted/40 p-2 font-mono text-xs" />
      </label>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="self-start"
        onClick={async () => {
          await navigator.clipboard.writeText(code).then(() => setCopied(true), () => undefined);
        }}
      >
        {copied ? "Copied" : "Copy code"}
      </Button>
    </div>
  );
}
