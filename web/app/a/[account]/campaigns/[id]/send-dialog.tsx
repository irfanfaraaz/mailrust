"use client";
import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Button, buttonVariants } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";
import { when } from "@/lib/format";
import { namesMissingText, wavePlanText } from "@/lib/report";
import { changeStatus, previewCampaign, setAudience } from "./actions";

type Props = { account: string; campaign: Schemas["Campaign"]; tags: Schemas["TagView"][]; beforeOpen?: () => Promise<void> };

export function SendDialog({ account, campaign, tags, beforeOpen }: Props) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [some, setSome] = useState(campaign.include_tags.length > 0);
  const [include, setInclude] = useState<string[]>(campaign.include_tags);
  const [exclude, setExclude] = useState<string[]>(campaign.exclude_tags);
  const [preview, setPreview] = useState<Schemas["CampaignPreview"] | null>(null);
  const [at, setAt] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const key = (inc: string[], exc: string[]) => JSON.stringify([[...inc].sort(), [...exc].sort()]);
  const saved = useRef(key(campaign.include_tags, campaign.exclude_tags));

  useEffect(() => {
    if (!open) return;
    let live = true;
    (async () => {
      await beforeOpen?.();
      const inc = some ? include : [];
      // Same audience: no PATCH (it would be a no-op, and a scheduled issue must stay scheduled).
      if (key(inc, exclude) !== saved.current) {
        const r = await setAudience(account, campaign.id, inc, exclude);
        if (r.error) return live && setError(r.error);
        saved.current = key(inc, exclude);
        if (campaign.status === "scheduled") router.refresh();
      }
      const p = await previewCampaign(account, campaign.id);
      if (live) setPreview(p);
    })();
    return () => { live = false; };
  }, [open, some, include, exclude, account, campaign.id, campaign.status, router, beforeOpen]);

  const toggle = (list: string[], set: (v: string[]) => void, id: string) => set(list.includes(id) ? list.filter((x) => x !== id) : [...list, id]);
  const ready = preview !== null && preview.checks.every((c) => c.ok);
  const go = async (sendAt?: string) => {
    setPending(true);
    await beforeOpen?.(); // pending edits go out with the send
    const r = await changeStatus(account, campaign.id, "send", sendAt);
    setPending(false);
    if (r.error) return setError(r.error);
    toast.success(sendAt ? `Scheduled for ${when(sendAt, Intl.DateTimeFormat().resolvedOptions().timeZone)}` : "Sending started");
    setOpen(false);
    router.refresh();
  };
  const missing = preview && namesMissingText(preview.missing_first_name, preview.readers, preview.first_name_fallback ?? null);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger className={buttonVariants({ size: "sm" })}>Send…</DialogTrigger>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Send this issue</DialogTitle>
          <DialogDescription>Only subscribed readers get it. Unsubscribed, pending and blocked addresses are always left out.</DialogDescription>
        </DialogHeader>
        <fieldset className="flex flex-col gap-2 text-sm">
          <legend className="mb-1 font-medium">Audience</legend>
          <label className="flex items-center gap-2"><input type="radio" name="audience" checked={!some} onChange={() => setSome(false)} />Everyone subscribed</label>
          <label className="flex items-center gap-2"><input type="radio" name="audience" checked={some} onChange={() => setSome(true)} />Only some tags</label>
          {some && (
            <div className="flex flex-wrap gap-3 pl-6" role="group" aria-label="Send to readers with any of these tags">
              {tags.map((t) => (
                <label key={t.id} className="flex items-center gap-1.5"><input type="checkbox" checked={include.includes(t.id)} onChange={() => toggle(include, setInclude, t.id)} />{t.name}</label>
              ))}
            </div>
          )}
          {tags.length > 0 && (
            <div className="flex flex-wrap gap-3" role="group" aria-label="Skip readers with any of these tags">
              <span className="text-muted-foreground">Skip:</span>
              {tags.map((t) => (
                <label key={t.id} className="flex items-center gap-1.5"><input type="checkbox" aria-label={`Skip ${t.name}`} checked={exclude.includes(t.id)} onChange={() => toggle(exclude, setExclude, t.id)} />{t.name}</label>
              ))}
            </div>
          )}
        </fieldset>
        {preview === null ? (
          <p className="text-sm text-muted-foreground">Checking…</p>
        ) : (
          <div className="flex flex-col gap-2 text-sm">
            <p>{wavePlanText(preview.waves)}</p>
            {missing && <p className="text-muted-foreground">{missing}</p>}
            <ul className="flex flex-col gap-1" aria-label="Checks">
              {preview.checks.map((c) => (
                <li key={c.code} className={c.ok ? "text-muted-foreground" : "text-destructive"}>{c.ok ? "✓" : "✗"} {c.message}</li>
              ))}
              {preview.warnings.map((w) => <li key={w} className="text-amber-700 dark:text-amber-300">! {w}</li>)}
            </ul>
          </div>
        )}
        {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
        <div className="flex flex-wrap items-end gap-3">
          <Button disabled={!ready || pending} onClick={() => go()}>Send now</Button>
          <label className="flex flex-col gap-1 text-sm">Send at
            <Input type="datetime-local" value={at} onChange={(e) => setAt(e.target.value)} />
          </label>
          <Button variant="outline" disabled={!ready || pending || !at} onClick={() => go(new Date(at).toISOString())}>Schedule</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
