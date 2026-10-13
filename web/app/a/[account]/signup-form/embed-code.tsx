"use client";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

export function EmbedCode({ code }: { code: string }) {
  const ref = useRef<HTMLTextAreaElement>(null);
  const [status, setStatus] = useState<"idle" | "copied" | "selected">("idle");
  useEffect(() => {
    if (status === "idle") return;
    const t = setTimeout(() => setStatus("idle"), 2000);
    return () => clearTimeout(t);
  }, [status]);
  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      setStatus("copied");
    } catch {
      // No clipboard access (http, old browser): select the code so Ctrl/Cmd+C works.
      ref.current?.focus();
      ref.current?.select();
      setStatus("selected");
    }
  }
  return (
    <div className="flex flex-col gap-2">
      <label className="flex flex-col gap-1 text-sm">Embed code
        <Textarea ref={ref} readOnly rows={7} value={code} onFocus={(e) => e.currentTarget.select()} className="bg-muted/40 font-mono md:text-xs" />
      </label>
      <div className="flex items-center gap-3">
        <Button type="button" variant="outline" size="sm" onClick={copy}>{status === "copied" ? "Copied" : "Copy code"}</Button>
        <span role="status" className="text-sm text-muted-foreground">{status === "selected" ? "Selected. Press Ctrl+C or Cmd+C to copy." : ""}</span>
      </div>
    </div>
  );
}
