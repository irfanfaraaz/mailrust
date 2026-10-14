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
    <form ref={form} action={action} className="grid gap-4 text-sm sm:grid-cols-2 lg:grid-cols-4">
      <label htmlFor="contact-email" className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Email</span>
        <Input id="contact-email" name="email" type="email" required />
      </label>
      <label htmlFor="contact-first" className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">First name</span>
        <Input id="contact-first" name="first_name" autoComplete="off" />
      </label>
      <label htmlFor="contact-last" className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Last name</span>
        <Input id="contact-last" name="last_name" autoComplete="off" />
      </label>
      <label htmlFor="contact-tags" className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Tags</span>
        <Input id="contact-tags" name="tags" placeholder="vip, news" />
      </label>
      <div className="sm:col-span-2 lg:col-span-4"><Button type="submit" disabled={pending}>Add contact</Button></div>
      {state.error && <p role="alert" className="text-destructive sm:col-span-2 lg:col-span-4">{state.error}</p>}
    </form>
  );
}
