"use client";
import { TriangleAlertIcon } from "lucide-react";
import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";

export function CopyOnce({ label, value }: { label: string; value: string }) {
  const ref = useRef<HTMLElement>(null);
  const [copied, setCopied] = useState(false);
  const noun = label.toLowerCase().includes("secret") ? "secret" : "key";
  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
    } catch {
      const r = document.createRange();
      if (ref.current) { r.selectNodeContents(ref.current); getSelection()?.removeAllRanges(); getSelection()?.addRange(r); }
    }
  }
  return (
    <div role="status" className="flex flex-col gap-3 rounded-lg border border-amber-500/40 bg-amber-500/10 p-4">
      <p className="text-sm font-medium">{label}</p>
      <code ref={ref} className="break-all rounded-md bg-background px-2 py-1.5 font-mono text-sm">{value}</code>
      <p className="flex items-start gap-2 text-sm font-medium text-amber-800 dark:text-amber-300">
        <TriangleAlertIcon aria-hidden className="mt-0.5 size-4 shrink-0" />
        <span>You will not see this {noun} again. Copy it now and store it somewhere safe.</span>
      </p>
      <Button type="button" className="self-start" onClick={copy}>{copied ? "Copied" : `Copy ${noun}`}</Button>
    </div>
  );
}
