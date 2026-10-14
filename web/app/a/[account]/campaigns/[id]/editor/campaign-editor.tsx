"use client";
import { EditorContent, useEditor, type JSONContent } from "@tiptap/react";
import { useRouter } from "next/navigation";
import StarterKit from "@tiptap/starter-kit";
import { Placeholder } from "@tiptap/extensions";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";
import { plain } from "@/lib/plain";
import { previewCampaign, saveDraft } from "../actions";
import { SendDialog } from "../send-dialog";
import { CampaignImage, EmailButton, MergeTag } from "./extensions";
import { InsertDialogs } from "./insert-dialogs";
import { PreviewDialog } from "./preview-dialog";
import { SlashHint } from "./slash-hint";
import { SlashCommand, type DialogKind } from "./slash-menu";
import { TestDialog } from "./test-dialog";

type Props = { account: string; campaign: Schemas["Campaign"]; me: string; tags: Schemas["TagView"][] };
type SaveState = "idle" | "saved" | "settled" | "saving" | { error: string };

/** A new issue is a doc with no blocks; give it one empty paragraph so the "type /" hint shows. */
const startDoc = (blocks: unknown): JSONContent => {
  const d = blocks as JSONContent;
  return d.content?.length ? d : { type: "doc", content: [{ type: "paragraph" }] };
};

export function CampaignEditor({ account, campaign, me, tags }: Props) {
  const router = useRouter();
  const [subject, setSubject] = useState(campaign.subject);
  const [previewText, setPreviewText] = useState(campaign.preview_text ?? "");
  const [doc, setDoc] = useState<JSONContent>(() => startDoc(campaign.blocks));
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [save, setSave] = useState<SaveState>("idle");
  const [preview, setPreview] = useState<Schemas["CampaignPreview"] | null>(null);
  const dirty = useRef(false);
  const savedSubject = useRef(campaign.subject);
  const subjectId = useId();
  const previewId = useId();

  const editor = useEditor({
    immediatelyRender: false,
    extensions: [
      StarterKit.configure({
        heading: { levels: [1, 2, 3] },
        code: false,
        codeBlock: false,
        strike: false,
        underline: false,
        link: { openOnClick: false, autolink: true, protocols: ["mailto"], defaultProtocol: "https" },
      }),
      Placeholder.configure({ placeholder: "Write, or type / for blocks" }),
      MergeTag,
      EmailButton,
      CampaignImage,
      SlashCommand.configure({ open: setDialog }),
    ],
    content: doc,
    editorProps: { attributes: { class: "campaign-editor min-h-[28rem] px-1 py-2 text-base focus:outline-none", "aria-label": "Issue body" } },
    onUpdate: ({ editor }) => {
      dirty.current = true;
      setDoc(editor.getJSON());
    },
  });

  // Nothing to save unless something changed: a save of a scheduled issue that changes it puts it back to draft.
  const flush = useCallback(async () => {
    if (!dirty.current) return;
    setSave("saving");
    const r = await saveDraft(account, campaign.id, { subject, preview_text: previewText, blocks: plain(doc) });
    setSave(r.error ? { error: r.error } : "saved");
    dirty.current = Boolean(r.error);
    if (r.error) return;
    // The page title and the badge come from the server: refresh when they changed.
    if (campaign.status === "scheduled" || subject !== savedSubject.current) router.refresh();
    savedSubject.current = subject;
  }, [account, campaign.id, campaign.status, router, subject, previewText, doc]);

  // Autosave 1.5 s after the last change.
  useEffect(() => {
    if (!dirty.current) return;
    const t = setTimeout(flush, 1500);
    return () => clearTimeout(t);
  }, [flush]);

  // "Saved" with its tick shows for 2 s, then settles into a quiet line.
  useEffect(() => {
    if (save !== "saved") return;
    const t = setTimeout(() => setSave("settled"), 2000);
    return () => clearTimeout(t);
  }, [save]);

  const edit = (f: (v: string) => void) => (e: React.ChangeEvent<HTMLInputElement>) => {
    dirty.current = true;
    f(e.target.value);
  };

  const audience = campaign.include_tags.length > 0 ? `Readers tagged ${campaign.include_tags.map((id) => tags.find((t) => t.id === id)?.name ?? id).join(", ")}` : "Everyone subscribed";
  const skipped = campaign.exclude_tags.map((id) => tags.find((t) => t.id === id)?.name ?? id);

  return (
    <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_19rem]">
      <div className="sketch sketch-lg flex flex-col gap-5 bg-card p-6 sm:p-8">
        <label htmlFor={subjectId} className="label-caps flex flex-col gap-1.5 text-muted-foreground">Subject
          <Input id={subjectId} value={subject} onChange={edit(setSubject)} maxLength={200} className="font-heading text-lg font-semibold normal-case tracking-normal text-foreground" />
        </label>
        <label htmlFor={previewId} className="label-caps flex flex-col gap-1.5 text-muted-foreground">Preview text
          <Input id={previewId} value={previewText} onChange={edit(setPreviewText)} maxLength={200} placeholder="Shown next to the subject in the inbox" className="normal-case tracking-normal text-foreground" />
        </label>
        <div role="toolbar" aria-label="Insert" className="flex flex-wrap gap-2 border-t border-dashed border-foreground/30 pt-5">
          <Button type="button" variant="outline" size="sm" onClick={() => setDialog("image")}>Image</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setDialog("button")}>Button</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setDialog("merge")}>Merge tag</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => editor?.chain().focus().setHorizontalRule().run()}>Divider</Button>
        </div>
        <div className="relative">
          <SlashHint empty={editor?.isEmpty ?? false} />
          <EditorContent editor={editor} />
        </div>
      </div>
      <aside aria-label="Issue actions" className="flex flex-col gap-5 lg:sticky lg:top-6">
        <Card>
          <CardHeader><h2 className="text-lg leading-snug font-semibold">Ready to send?</h2></CardHeader>
          <CardContent className="flex flex-col gap-3">
            <SendDialog account={account} campaign={campaign} tags={tags} beforeOpen={flush} />
            <Button type="button" variant="outline" onClick={async () => { await flush(); setPreview(await previewCampaign(account, campaign.id)); }}>Preview</Button>
            <TestDialog account={account} id={campaign.id} me={me} beforeSend={flush} />
            <span role="status" className="text-sm text-muted-foreground">
              {save === "idle" ? "" : save === "saving" ? (
                <span className="inline-flex items-center gap-2"><span aria-hidden className="save-dot size-1.5 rounded-full bg-wait" />Saving…</span>
              ) : save === "saved" ? (
                <span className="inline-flex items-center gap-1.5 text-good">
                  <svg aria-hidden className="tick-draw size-4" viewBox="0 0 16 16"><path d="M3 8.5 6.5 12 13 4.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" /></svg>
                  Saved
                </span>
              ) : save === "settled" ? "All changes saved" : <span className="text-destructive">Not saved: {save.error}</span>}
            </span>
          </CardContent>
        </Card>
        <Card>
          <CardHeader><h2 className="text-lg leading-snug font-semibold">Audience</h2></CardHeader>
          <CardContent className="flex flex-col gap-1.5">
            <span>{audience}</span>
            {skipped.length > 0 && <span className="text-muted-foreground">Skipping {skipped.join(", ")}</span>}
            <span className="text-muted-foreground">Choose the audience and see the checks when you press Send.</span>
          </CardContent>
        </Card>
      </aside>
      <InsertDialogs account={account} editor={editor} kind={dialog} close={() => setDialog(null)} />
      <PreviewDialog preview={preview} close={() => setPreview(null)} />
    </div>
  );
}
