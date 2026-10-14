"use client";
import Link from "next/link";
import { useActionState, useEffect } from "react";
import { toast } from "sonner";
import { NativeSelect } from "@/components/native-select";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
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
    <form action={action} className="flex max-w-xl flex-col gap-5 text-sm">
      <input type="hidden" name="from_current" value={current} />
      <Card>
        <CardHeader>
          <h2 className="text-lg leading-snug font-semibold">Your page</h2>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Public address</span>
        {/* On a phone the long origin sits above the field instead of squeezing it. */}
        <span className="flex flex-col gap-1 sm:flex-row sm:items-center">
          <span className="break-all text-muted-foreground sm:shrink-0 sm:break-normal">{origin}/join/</span>
          <Input name="public_slug" defaultValue={settings.public_slug ?? ""} placeholder="weekly" />
        </span>
      </label>
      <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Newsletter name</span>
        <Input name="newsletter_name" defaultValue={settings.newsletter_name ?? ""} placeholder="Shop Weekly" />
      </label>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <h2 className="text-lg leading-snug font-semibold">Sending</h2>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
      {domains.length === 0 ? (
        <p className="text-muted-foreground">
          {current && <>Confirm emails come from {current}, but {domain} is not verified any more. </>}
          Confirm emails go out from one of your verified domains. <Link className="text-primary underline underline-offset-2" href={`/a/${account}/domains`}>Verify a domain</Link> first.
        </p>
      ) : (
        <div className="flex flex-col gap-1.5">
          <span className="label-caps text-muted-foreground">Emails come from</span>
          <span className="flex items-center gap-1">
            <Input aria-label="From address" name="from_local" defaultValue={local} placeholder="hello" />
            <span aria-hidden>@</span>
            <NativeSelect aria-label="From domain" name="from_domain" defaultValue={domain || domains[0]}>
              {domainOptions.map((d) => <option key={d} value={d}>{d === domain && unverified ? `${d} (not verified)` : d}</option>)}
            </NativeSelect>
          </span>
          <p className="text-muted-foreground">Confirm emails and every issue go out from this address.</p>
          {unverified && (
            <p className="text-muted-foreground">
              {domain} is not verified any more, so the form takes no signups. <Link className="text-primary underline underline-offset-2" href={`/a/${account}/domains`}>Verify it again</Link> or pick another domain.
            </p>
          )}
        </div>
      )}
      <div className="flex flex-col gap-1.5">
        <label htmlFor="postal_address" className="label-caps text-muted-foreground">Postal address</label>
        <Textarea
          id="postal_address"
          name="postal_address"
          rows={3}
          maxLength={300}
          defaultValue={settings.postal_address ?? ""}
          placeholder={"Shop Ltd\n1 Main Street\nLondon"}
          aria-describedby="postal-hint"
        />
        <p id="postal-hint" className="text-muted-foreground">Shown at the bottom of every issue. Most countries require it, and issues cannot be sent without it.</p>
      </div>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <h2 className="text-lg leading-snug font-semibold">Signups</h2>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
      <label className="flex items-center gap-2">
        <input type="checkbox" name="signup_enabled" defaultChecked={settings.signup_enabled} className="size-4 accent-primary" />
        Take signups
      </label>
      <label className="flex items-center gap-2">
        <input type="checkbox" name="signup_ask_name" defaultChecked={settings.signup_ask_name} className="size-4 accent-primary" />
        Ask for first name
      </label>
        </CardContent>
      </Card>
      {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
      <Button type="submit" disabled={pending} className="self-start">Save</Button>
    </form>
  );
}
