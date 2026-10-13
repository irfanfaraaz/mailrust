"use client";
import { useActionState, useEffect } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";
import { formatFields } from "@/lib/fields";
import { updateContact, type ContactState } from "../actions";

const none: ContactState = {};

export function EditContact({ account, contact }: { account: string; contact: Schemas["ContactView"] }) {
  const [state, action, pending] = useActionState(updateContact.bind(null, account, contact.id), none);
  useEffect(() => {
    if (state !== none && !state.error) toast.success("Contact saved");
  }, [state]);
  return (
    <form action={action} className="flex max-w-xl flex-col gap-3 text-sm">
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1">First name
          <Input name="first_name" defaultValue={contact.first_name ?? ""} />
        </label>
        <label className="flex flex-col gap-1">Last name
          <Input name="last_name" defaultValue={contact.last_name ?? ""} />
        </label>
      </div>
      <label className="flex flex-col gap-1">Tags
        <Input name="tags" defaultValue={contact.tags.map((t) => t.name).join(", ")} placeholder="vip, news" />
      </label>
      <label className="flex flex-col gap-1">Fields
        <textarea name="fields" rows={5} defaultValue={formatFields(contact.fields)} placeholder={"plan=pro\ncity=Pune"} className="rounded-lg border bg-transparent p-2 font-mono text-xs" />
        <span className="text-xs text-muted-foreground">One name=value per line. Names use a-z, 0-9 and _.</span>
      </label>
      {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
      <Button type="submit" disabled={pending} className="self-start">Save</Button>
    </form>
  );
}
