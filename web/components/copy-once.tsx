"use client";
import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";

export function CopyOnce({ label, value }: { label: string; value: string }) {
  const ref = useRef<HTMLElement>(null);
  const [copied, setCopied] = useState(false);
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
    <div role="status" className="flex flex-col gap-2 rounded border p-3">
      <p className="text-sm font-medium">{label}</p>
      <code ref={ref} className="break-all text-sm">{value}</code>
      <div className="flex items-center gap-3">
        <Button type="button" variant="secondary" onClick={copy}>{copied ? "Copied" : "Copy"}</Button>
        <span className="text-sm text-muted-foreground">You will not see this {label.toLowerCase().includes("secret") ? "secret" : "key"} again.</span>
      </div>
    </div>
  );
}
