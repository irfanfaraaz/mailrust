"use client";
import { useActionState, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { createTag, renameTag, type TagState } from "./actions";

const none: TagState = {};

export function AddTag({ account }: { account: string }) {
  const [state, action, pending] = useActionState(createTag.bind(null, account), none);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    if (state !== none && !state.error) {
      toast.success("Tag added");
      form.current?.reset();
    }
  }, [state]);
  return (
    <form ref={form} action={action} className="flex flex-wrap items-end gap-3 text-sm">
      <label className="flex flex-col gap-1">New tag
        <Input id="new-tag" name="name" required maxLength={50} aria-invalid={state.error ? true : undefined} aria-describedby={state.error ? "new-tag-error" : undefined} />
      </label>
      <Button type="submit" disabled={pending}>Add tag</Button>
      {state.error && <p id="new-tag-error" role="alert" className="basis-full text-destructive">{state.error}</p>}
    </form>
  );
}

/** The name as text with a quiet Rename; the field only appears for the row being renamed. */
export function RenameTag({ account, id, name }: { account: string; id: string; name: string }) {
  const [editing, setEditing] = useState(false);
  const [state, action, pending] = useActionState(async (prev: TagState, form: FormData) => {
    const r = await renameTag(account, id, prev, form);
    if (!r.error) {
      toast.success("Tag renamed");
      setEditing(false);
    }
    return r;
  }, none);
  const trigger = useRef<HTMLButtonElement>(null);
  const errorId = `rename-${id}-error`;
  // Back on the Rename button after Save or Cancel, so keyboard users keep their place.
  const wasEditing = useRef(false);
  useEffect(() => {
    if (!editing && wasEditing.current) trigger.current?.focus();
    wasEditing.current = editing;
  }, [editing]);
  if (!editing) {
    return (
      <span className="flex flex-wrap items-center gap-2">
        <span className="font-medium">{name}</span>
        <Button ref={trigger} type="button" variant="ghost" size="sm" aria-label={`Rename ${name}`} onClick={() => setEditing(true)}>Rename</Button>
      </span>
    );
  }
  return (
    <form
      action={action}
      className="flex flex-wrap items-center gap-2"
      onKeyDown={(e) => { if (e.key === "Escape") setEditing(false); }}
    >
      <Input
        autoFocus
        aria-label={`New name for ${name}`}
        name="name"
        defaultValue={name}
        required
        maxLength={50}
        className="max-w-56"
        aria-invalid={state.error ? true : undefined}
        aria-describedby={state.error ? errorId : undefined}
      />
      <Button type="submit" size="sm" disabled={pending}>Save</Button>
      <Button type="button" variant="ghost" size="sm" onClick={() => setEditing(false)}>Cancel</Button>
      {state.error && <p id={errorId} role="alert" className="basis-full text-sm text-destructive">{state.error}</p>}
    </form>
  );
}
