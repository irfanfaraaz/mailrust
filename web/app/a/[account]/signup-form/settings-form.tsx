"use client";
import Link from "next/link";
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
  const current = settings.newsletter_from ?? "";
  const [local = "", domain = ""] = current.split("@");
  // The saved domain lost its verification: keep showing it (and keep it on save) instead of swapping in another.
  const unverified = domain !== "" && !domains.includes(domain);
  const domainOptions = unverified ? [domain, ...domains] : domains;
  return (
    <form action={action} className="flex max-w-xl flex-col gap-4 text-sm">
      <input type="hidden" name="from_current" value={current} />
      <label className="flex flex-col gap-1">Public address
        {/* On a phone the long origin sits above the field instead of squeezing it. */}
        <span className="flex flex-col gap-1 sm:flex-row sm:items-center">
          <span className="break-all text-muted-foreground sm:shrink-0 sm:break-normal">{origin}/join/</span>
          <Input name="public_slug" defaultValue={settings.public_slug ?? ""} placeholder="weekly" />
        </span>
      </label>
      <label className="flex flex-col gap-1">Newsletter name
        <Input name="newsletter_name" defaultValue={settings.newsletter_name ?? ""} placeholder="Shop Weekly" />
      </label>
      {domains.length === 0 ? (
        <p className="text-muted-foreground">
          {current && <>Confirm emails come from {current}, but {domain} is not verified any more. </>}
          Confirm emails go out from one of your verified domains. <Link className="underline underline-offset-2" href={`/a/${account}/domains`}>Verify a domain</Link> first.
        </p>
      ) : (
        <div className="flex flex-col gap-1">
          <span>Confirm emails come from</span>
          <span className="flex items-center gap-1">
            <Input aria-label="From address" name="from_local" defaultValue={local} placeholder="hello" />
            <span aria-hidden>@</span>
            <NativeSelect aria-label="From domain" name="from_domain" defaultValue={domain || domains[0]}>
              {domainOptions.map((d) => <option key={d} value={d}>{d === domain && unverified ? `${d} (not verified)` : d}</option>)}
            </NativeSelect>
          </span>
          {unverified && (
            <p className="text-muted-foreground">
              {domain} is not verified any more, so the form takes no signups. <Link className="underline underline-offset-2" href={`/a/${account}/domains`}>Verify it again</Link> or pick another domain.
            </p>
          )}
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
