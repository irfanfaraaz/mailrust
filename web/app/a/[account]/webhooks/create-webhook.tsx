"use client";
import { useActionState } from "react";
import { CopyOnce } from "@/components/copy-once";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { createWebhook, type CreateWebhookState } from "./actions";

const EVENTS = ["email.sent", "email.delivered", "email.delivery_delayed", "email.bounced", "email.complained", "email.opened", "email.clicked", "email.failed", "email.suppressed", "account.paused"];

export function CreateWebhook({ account }: { account: string }) {
  const [state, action, pending] = useActionState(createWebhook.bind(null, account), {} as CreateWebhookState);
  return (
    <div className="flex flex-col gap-5">
      {state.created?.secret && <CopyOnce label="Signing secret" value={state.created.secret} />}
      <Card>
        <CardHeader>
          <h2 className="text-lg leading-snug font-semibold">New webhook</h2>
        </CardHeader>
        <CardContent>
          <form action={action} className="flex flex-col gap-4 text-sm">
            <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">URL</span>
              <Input id="webhook-url" name="url" type="url" required placeholder="https://example.com/hook" className="font-mono" />
            </label>
            <fieldset className="flex flex-wrap gap-x-5 gap-y-1">
              <legend className="label-caps mb-2 text-muted-foreground">Events</legend>
              {EVENTS.map((e) => (
                <label key={e} className="flex items-center gap-1.5 font-mono text-[0.8125rem] max-md:min-h-11">
                  <input type="checkbox" name="events" value={e} className="accent-primary" />{e}
                </label>
              ))}
            </fieldset>
            <Button type="submit" className="self-start" disabled={pending}>Add webhook</Button>
          </form>
          {state.error && <p role="alert" className="mt-3 text-sm text-destructive">{state.error}</p>}
        </CardContent>
      </Card>
    </div>
  );
}
