"use client";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import type { Schemas } from "@/lib/engine";

/** The engine's own render in a sandboxed frame (no scripts, no same-origin). */
export function PreviewDialog({ preview, close }: { preview: Schemas["CampaignPreview"] | null; close: () => void }) {
  const [width, setWidth] = useState<600 | 375>(600);
  return (
    <Dialog open={preview !== null} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader><DialogTitle>Preview</DialogTitle></DialogHeader>
        <div className="flex gap-2">
          <Button variant={width === 600 ? "default" : "outline"} size="sm" onClick={() => setWidth(600)}>Desktop</Button>
          <Button variant={width === 375 ? "default" : "outline"} size="sm" onClick={() => setWidth(375)}>Phone</Button>
        </div>
        {/* Never add allow-scripts: same-origin alone only lets the frame load /img images, which a fully opaque frame blocks. */}
        {preview && <iframe title="Email preview" sandbox="allow-same-origin" srcDoc={preview.html} width={width} height={560} className="max-w-full rounded-md border bg-white" />}
        <p className="text-sm text-muted-foreground">Merge tags show their fallbacks here; each reader sees their own values.</p>
      </DialogContent>
    </Dialog>
  );
}
