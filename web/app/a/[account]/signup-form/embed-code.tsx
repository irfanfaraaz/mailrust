"use client";
import { useEffect, useRef, useState } from "react";
import { CopiedHint } from "@/components/copied-hint";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

export function EmbedCode({ code }: { code: string }) {
  const ref = useRef<HTMLTextAreaElement>(null);
  const [status, setStatus] = useState<"idle" | "copied" | "selected">("idle");
  const [copies, setCopies] = useState(0);
  useEffect(() => {
    if (status === "idle") return;
    const t = setTimeout(() => setStatus("idle"), 2000);
    return () => clearTimeout(t);
  }, [status]);
  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      setStatus("copied");
      setCopies((n) => n + 1);
    } catch {
      // No clipboard access (http, old browser): select the code so Ctrl/Cmd+C works.
      ref.current?.focus();
      ref.current?.select();
      setStatus("selected");
    }
  }
  return (
    <div className="flex flex-col gap-2">
      <label className="flex flex-col gap-1.5 text-sm"><span className="label-caps text-muted-foreground">Embed code</span>
        <Textarea ref={ref} readOnly rows={7} value={code} onFocus={(e) => e.currentTarget.select()} className="bg-foreground/[0.04] font-mono leading-relaxed md:text-xs" />
      </label>
      <div className="flex items-center gap-3">
        <span className="relative inline-flex">
          <Button type="button" variant="outline" size="sm" onClick={copy}>{status === "copied" ? "Copied" : "Copy code"}</Button>
          <CopiedHint n={copies} />
        </span>
        <span role="status" className="text-sm text-muted-foreground">{status === "selected" ? "Selected. Press Ctrl+C or Cmd+C to copy." : ""}</span>
      </div>
    </div>
  );
}
