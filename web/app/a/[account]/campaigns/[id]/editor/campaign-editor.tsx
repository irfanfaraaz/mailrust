"use client";
import { EditorContent, useEditor, type JSONContent } from "@tiptap/react";
import { useRouter } from "next/navigation";
import StarterKit from "@tiptap/starter-kit";
import { Placeholder } from "@tiptap/extensions";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";
import { previewCampaign, saveDraft } from "../actions";
import { SendDialog } from "../send-dialog";
import { CampaignImage, EmailButton, MergeTag } from "./extensions";
import { InsertDialogs } from "./insert-dialogs";
import { PreviewDialog } from "./preview-dialog";
import { SlashCommand, type DialogKind } from "./slash-menu";
import { TestDialog } from "./test-dialog";

type Props = { account: string; campaign: Schemas["Campaign"]; me: string; tags: Schemas["TagView"][] };
type SaveState = "idle" | "saved" | "saving" | { error: string };

export function CampaignEditor({ account, campaign, me, tags }: Props) {
  const router = useRouter();
  const [subject, setSubject] = useState(campaign.subject);
  const [previewText, setPreviewText] = useState(campaign.preview_text ?? "");
  const [doc, setDoc] = useState<JSONContent>(campaign.blocks as JSONContent);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [save, setSave] = useState<SaveState>("idle");
  const [preview, setPreview] = useState<Schemas["CampaignPreview"] | null>(null);
  const dirty = useRef(false);

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
    content: campaign.blocks as JSONContent,
    editorProps: { attributes: { class: "campaign-editor min-h-80 rounded-md border px-4 py-3 focus:outline-none", "aria-label": "Issue body" } },
    onUpdate: ({ editor }) => {
      dirty.current = true;
      setDoc(editor.getJSON());
    },
  });

  // Nothing to save unless something changed: a save of a scheduled issue that changes it puts it back to draft.
  const flush = useCallback(async () => {
    if (!dirty.current) return;
    setSave("saving");
    const r = await saveDraft(account, campaign.id, { subject, preview_text: previewText, blocks: doc });
    setSave(r.error ? { error: r.error } : "saved");
    dirty.current = Boolean(r.error);
    if (!r.error && campaign.status === "scheduled") router.refresh(); // the badge follows the engine
  }, [account, campaign.id, campaign.status, router, subject, previewText, doc]);

  // Autosave 1.5 s after the last change.
  useEffect(() => {
    if (!dirty.current) return;
    const t = setTimeout(flush, 1500);
    return () => clearTimeout(t);
  }, [flush]);

  const edit = (f: (v: string) => void) => (e: React.ChangeEvent<HTMLInputElement>) => {
    dirty.current = true;
    f(e.target.value);
  };

  return (
    <div className="flex flex-col gap-4">
      <label className="flex flex-col gap-1 text-sm">Subject
        <Input value={subject} onChange={edit(setSubject)} maxLength={200} className="text-base" />
      </label>
      <label className="flex flex-col gap-1 text-sm">Preview text
        <Input value={previewText} onChange={edit(setPreviewText)} maxLength={200} placeholder="Shown next to the subject in the inbox" />
      </label>
      <div role="toolbar" aria-label="Insert" className="flex flex-wrap gap-2">
        <Button type="button" variant="outline" size="sm" onClick={() => setDialog("image")}>Image</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => setDialog("button")}>Button</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => setDialog("merge")}>Merge tag</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => editor?.chain().focus().setHorizontalRule().run()}>Divider</Button>
      </div>
      <EditorContent editor={editor} />
      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" variant="outline" size="sm" onClick={async () => { await flush(); setPreview(await previewCampaign(account, campaign.id)); }}>Preview</Button>
        <TestDialog account={account} id={campaign.id} me={me} beforeSend={flush} />
        <SendDialog account={account} campaign={campaign} tags={tags} beforeOpen={flush} />
        <span role="status" className="ml-auto text-sm text-muted-foreground">
          {save === "idle" ? "" : save === "saved" ? "Saved" : save === "saving" ? "Saving…" : <span className="text-destructive">Not saved: {save.error}</span>}
        </span>
      </div>
      <InsertDialogs account={account} editor={editor} kind={dialog} close={() => setDialog(null)} />
      <PreviewDialog preview={preview} close={() => setPreview(null)} />
    </div>
  );
}
