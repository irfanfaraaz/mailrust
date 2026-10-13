"use client";
import type { Editor } from "@tiptap/core";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { BUILT_IN_KEYS, MERGE_LABELS, mergeTagProblem } from "@/lib/merge-tags";
import { uploadImage } from "../actions";
import type { DialogKind } from "./slash-menu";

type Props = { account: string; editor: Editor | null; kind: DialogKind | null; close: () => void };

export function InsertDialogs({ account, editor, kind, close }: Props) {
  return (
    <Dialog open={kind !== null} onOpenChange={(o) => !o && close()}>
      <DialogContent>
        {kind === "merge" && <MergeForm editor={editor} close={close} />}
        {kind === "button" && <ButtonForm editor={editor} close={close} />}
        {kind === "image" && <ImageForm account={account} editor={editor} close={close} />}
      </DialogContent>
    </Dialog>
  );
}

function MergeForm({ editor, close }: { editor: Editor | null; close: () => void }) {
  const [key, setKey] = useState("first_name");
  const [custom, setCustom] = useState("");
  const [fallback, setFallback] = useState("there");
  const [error, setError] = useState<string | null>(null);
  const realKey = key === "custom" ? custom.trim() : key;
  return (
    <form
      className="flex flex-col gap-3 text-sm"
      onSubmit={(e) => {
        e.preventDefault();
        const problem = mergeTagProblem(realKey, fallback);
        if (problem) return setError(problem);
        editor?.chain().focus().insertContent({ type: "mergeTag", attrs: { key: realKey, fallback: fallback.trim() } }).run();
        close();
      }}
    >
      <DialogHeader><DialogTitle>Merge tag</DialogTitle></DialogHeader>
      <label className="flex flex-col gap-1">Value
        <NativeSelect value={key} onChange={(e) => setKey(e.target.value)}>
          {BUILT_IN_KEYS.map((k) => <option key={k} value={k}>{MERGE_LABELS[k]}</option>)}
          <option value="custom">A contact field…</option>
        </NativeSelect>
      </label>
      {key === "custom" && <label className="flex flex-col gap-1">Field name<Input value={custom} onChange={(e) => setCustom(e.target.value)} placeholder="plan" /></label>}
      <label className="flex flex-col gap-1">Fallback
        <Input value={fallback} onChange={(e) => setFallback(e.target.value)} maxLength={100} />
        <span className="text-muted-foreground">Readers without this value see the fallback.</span>
      </label>
      {error && <p role="alert" className="text-destructive">{error}</p>}
      <Button type="submit" className="self-start">Insert</Button>
    </form>
  );
}

function ButtonForm({ editor, close }: { editor: Editor | null; close: () => void }) {
  const [error, setError] = useState<string | null>(null);
  return (
    <form
      className="flex flex-col gap-3 text-sm"
      onSubmit={(e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const label = String(f.get("label") ?? "").trim();
        const href = String(f.get("href") ?? "").trim();
        if (!label) return setError("Add a label.");
        if (!/^(https?:\/\/\S+|mailto:\S+)$/i.test(href)) return setError("Links start with https://, http:// or mailto:.");
        editor?.chain().focus().insertContent({ type: "button", attrs: { label, href } }).run();
        close();
      }}
    >
      <DialogHeader><DialogTitle>Button</DialogTitle></DialogHeader>
      <label className="flex flex-col gap-1">Label<Input name="label" defaultValue="Read more" /></label>
      <label className="flex flex-col gap-1">Link<Input name="href" type="url" placeholder="https://" /></label>
      {error && <p role="alert" className="text-destructive">{error}</p>}
      <Button type="submit" className="self-start">Insert</Button>
    </form>
  );
}

function ImageForm({ account, editor, close }: { account: string; editor: Editor | null; close: () => void }) {
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  return (
    <form
      className="flex flex-col gap-3 text-sm"
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const alt = String(f.get("alt") ?? "").trim();
        const href = String(f.get("href") ?? "").trim();
        if (!alt) return setError("Describe the image for people who cannot see it.");
        if (href && !/^https?:\/\/\S+$/i.test(href)) return setError("Links start with https:// or http://.");
        setPending(true);
        const up = new FormData();
        up.set("file", f.get("file") as File);
        const r = await uploadImage(account, up);
        setPending(false);
        if ("error" in r) return setError(r.error);
        editor?.chain().focus().insertContent({ type: "image", attrs: { id: r.id, alt, href: href || null } }).run();
        close();
      }}
    >
      <DialogHeader><DialogTitle>Image</DialogTitle></DialogHeader>
      <label className="flex flex-col gap-1">Image file<Input name="file" type="file" accept="image/png,image/jpeg,image/gif,image/webp" required /></label>
      <label className="flex flex-col gap-1">Alt text<Input name="alt" /></label>
      <label className="flex flex-col gap-1">Link (optional)<Input name="href" type="url" placeholder="https://" /></label>
      {error && <p role="alert" className="text-destructive">{error}</p>}
      <Button type="submit" disabled={pending} className="self-start">{pending ? "Uploading…" : "Insert"}</Button>
    </form>
  );
}
