"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { createTag, renameTag, type TagState } from "./actions";

const none: TagState = {};

export function AddTag({ account }: { account: string }) {
  const [state, action, pending] = useActionState(createTag.bind(null, account), none);
  return (
    <form action={action} className="flex flex-wrap items-end gap-3 text-sm">
      <label className="flex flex-col gap-1">New tag
        <Input name="name" required maxLength={50} />
      </label>
      <Button type="submit" disabled={pending}>Add tag</Button>
      {state.error && <p role="alert" className="basis-full text-destructive">{state.error}</p>}
    </form>
  );
}

export function RenameTag({ account, id, name }: { account: string; id: string; name: string }) {
  const [state, action, pending] = useActionState(renameTag.bind(null, account, id), none);
  return (
    <form action={action} className="flex flex-wrap items-center gap-2">
      <Input aria-label="Name" name="name" defaultValue={name} required maxLength={50} className="max-w-56" />
      <Button type="submit" variant="outline" size="sm" disabled={pending}>Rename</Button>
      {state.error && <p role="alert" className="basis-full text-sm text-destructive">{state.error}</p>}
    </form>
  );
}
