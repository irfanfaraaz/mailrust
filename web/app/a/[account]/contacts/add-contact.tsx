"use client";
import { useActionState, useEffect, useRef } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { addContact, type ContactState } from "./actions";

const none: ContactState = {};

/** Adds a subscribed contact at once (no confirm email): for people who already agreed elsewhere. */
export function AddContact({ account }: { account: string }) {
  const [state, action, pending] = useActionState(addContact.bind(null, account), none);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    if (state !== none && !state.error) {
      toast.success("Contact saved");
      form.current?.reset();
    }
  }, [state]);
  return (
    <form ref={form} action={action} className="flex flex-wrap items-end gap-3 text-sm">
      <label htmlFor="contact-email" className="flex flex-col gap-1">Email
        <Input id="contact-email" name="email" type="email" required />
      </label>
      <label htmlFor="contact-first" className="flex flex-col gap-1">First name
        <Input id="contact-first" name="first_name" autoComplete="off" />
      </label>
      <label htmlFor="contact-last" className="flex flex-col gap-1">Last name
        <Input id="contact-last" name="last_name" autoComplete="off" />
      </label>
      <label htmlFor="contact-tags" className="flex flex-col gap-1">Tags
        <Input id="contact-tags" name="tags" placeholder="vip, news" />
      </label>
      <Button type="submit" disabled={pending}>Add contact</Button>
      {state.error && <p role="alert" className="basis-full text-destructive">{state.error}</p>}
    </form>
  );
}
