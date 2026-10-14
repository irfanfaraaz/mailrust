"use client";
import { TriangleAlertIcon } from "lucide-react";
import { useRef, useState } from "react";
import { CopiedHint } from "@/components/copied-hint";
import { Button } from "@/components/ui/button";

export function CopyOnce({ label, value }: { label: string; value: string }) {
  const ref = useRef<HTMLElement>(null);
  const [copied, setCopied] = useState(0);
  const noun = label.toLowerCase().includes("secret") ? "secret" : "key";
  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied((n) => n + 1);
    } catch {
      const r = document.createRange();
      if (ref.current) { r.selectNodeContents(ref.current); getSelection()?.removeAllRanges(); getSelection()?.addRange(r); }
    }
  }
  return (
    <div role="status" className="sketch sketch-warn flex flex-col gap-3 bg-card p-5">
      <p className="label-caps text-muted-foreground">{label}</p>
      <code ref={ref} className="break-all rounded-md border-[1.5px] border-dashed border-foreground/40 bg-background px-3 py-2 font-mono text-sm">{value}</code>
      <p className="flex items-start gap-2 text-sm font-medium text-wait">
        <TriangleAlertIcon aria-hidden className="mt-0.5 size-4 shrink-0" />
        <span>You will not see this {noun} again. Copy it now and store it somewhere safe.</span>
      </p>
      <span className="relative self-start">
        <Button type="button" onClick={copy}>{copied ? "Copied" : `Copy ${noun}`}</Button>
        <CopiedHint n={copied} />
      </span>
    </div>
  );
}
