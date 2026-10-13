"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { unsubscribeAction, type UnsubState } from "./actions";

/** `initial`: where a no-JavaScript submit left off (see noJsUnsubState). */
export function UnsubscribeForm({ token, name, initial }: { token: string; name: string; initial: UnsubState }) {
  const [state, action, pending] = useActionState(unsubscribeAction.bind(null, token), initial);
  if (state.step === "resubscribed") return <p role="status" className="text-lg">You&apos;re subscribed to {name} again.</p>;
  if (state.step === "unsubscribed") {
    return (
      <form action={action} className="flex flex-col gap-3">
        <p role="status" className="text-lg">You&apos;re unsubscribed from {name}.</p>
        <p className="text-sm text-muted-foreground">Clicked by mistake?</p>
        {state.error && <p role="alert" className="text-sm text-destructive">{state.error}</p>}
        <input type="hidden" name="do" value="resubscribe" />
        <Button type="submit" variant="outline" disabled={pending} className="self-start">Resubscribe</Button>
      </form>
    );
  }
  return (
    <form action={action} className="flex flex-col gap-3">
      <p>Stop getting {name}?</p>
      <input type="hidden" name="do" value="unsubscribe" />
      <Button type="submit" disabled={pending} className="self-start">Unsubscribe</Button>
    </form>
  );
}
