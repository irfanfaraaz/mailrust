"use client";
import { useActionState, useEffect } from "react";
import { toast } from "sonner";
import { NativeSelect } from "@/components/native-select";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Schemas } from "@/lib/engine";
import { saveNewsletter, type SaveState } from "./actions";

const none: SaveState = {};

type Props = { account: string; settings: Schemas["NewsletterSettings"]; domains: string[]; origin: string };

export function SettingsForm({ account, settings, domains, origin }: Props) {
  const [state, action, pending] = useActionState(saveNewsletter.bind(null, account), none);
  useEffect(() => {
    if (state !== none && !state.error) toast.success("Signup form saved");
  }, [state]);
  const [local = "", domain = ""] = (settings.newsletter_from ?? "").split("@");
  return (
    <form action={action} className="flex max-w-xl flex-col gap-4 text-sm">
      <label className="flex flex-col gap-1">Public address
        <span className="flex items-center gap-1">
          <span className="shrink-0 text-muted-foreground">{origin}/join/</span>
          <Input name="public_slug" defaultValue={settings.public_slug ?? ""} placeholder="weekly" />
        </span>
      </label>
      <label className="flex flex-col gap-1">Newsletter name
        <Input name="newsletter_name" defaultValue={settings.newsletter_name ?? ""} placeholder="Shop Weekly" />
      </label>
      {domains.length === 0 ? (
        <p className="text-muted-foreground">
          Confirm emails go out from one of your verified domains. <a className="underline" href={`/a/${account}/domains`}>Verify a domain</a> first.
        </p>
      ) : (
        <div className="flex flex-col gap-1">
          <span>Confirm emails come from</span>
          <span className="flex items-center gap-1">
            <Input aria-label="From address" name="from_local" defaultValue={local} placeholder="hello" />
            <span aria-hidden>@</span>
            <NativeSelect aria-label="From domain" name="from_domain" defaultValue={domains.includes(domain) ? domain : domains[0]}>
              {domains.map((d) => <option key={d} value={d}>{d}</option>)}
            </NativeSelect>
          </span>
        </div>
      )}
      <label className="flex items-center gap-2">
        <input type="checkbox" name="signup_enabled" defaultChecked={settings.signup_enabled} className="size-4" />
        Take signups
      </label>
      {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
      <Button type="submit" disabled={pending} className="self-start">Save</Button>
    </form>
  );
}
