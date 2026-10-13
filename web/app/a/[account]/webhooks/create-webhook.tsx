"use client";
import { useActionState } from "react";
import { CopyOnce } from "@/components/copy-once";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { createWebhook, type CreateWebhookState } from "./actions";

const EVENTS = ["email.sent", "email.delivered", "email.delivery_delayed", "email.bounced", "email.complained", "email.opened", "email.clicked", "email.failed", "email.suppressed", "account.paused"];

export function CreateWebhook({ account }: { account: string }) {
  const [state, action, pending] = useActionState(createWebhook.bind(null, account), {} as CreateWebhookState);
  return (
    <div className="flex flex-col gap-4">
      <form action={action} className="flex flex-col gap-3 text-sm">
        <label className="flex flex-col gap-1">URL
          <Input id="webhook-url" name="url" type="url" required placeholder="https://example.com/hook" />
        </label>
        <fieldset className="flex flex-wrap gap-x-4 gap-y-1">
          <legend className="mb-1">Events</legend>
          {EVENTS.map((e) => (
            <label key={e} className="flex items-center gap-1.5 max-md:min-h-11">
              <input type="checkbox" name="events" value={e} />{e}
            </label>
          ))}
        </fieldset>
        <Button type="submit" className="self-start" disabled={pending}>Add webhook</Button>
      </form>
      {state.error && <p role="alert" className="text-sm text-destructive">{state.error}</p>}
      {state.created?.secret && <CopyOnce label="Signing secret" value={state.created.secret} />}
    </div>
  );
}
