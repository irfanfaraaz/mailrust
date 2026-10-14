"use client";
import { useActionState } from "react";
import { Stamp } from "@/components/stamp";
import { Button } from "@/components/ui/button";
import { unsubscribeAction, type UnsubState } from "./actions";

/** `initial`: where a no-JavaScript submit left off (see noJsUnsubState). */
export function UnsubscribeForm({ token, name, initial }: { token: string; name: string; initial: UnsubState }) {
  const [state, action, pending] = useActionState(unsubscribeAction.bind(null, token), initial);
  if (state.step === "unsubscribed" || state.step === "resubscribed") {
    const back = state.step === "resubscribed";
    // One stamp in one place, so it lands again when "Unsubscribed" turns into "Subscribed".
    return (
      <form action={action} className="flex flex-col items-start gap-3">
        <span className="stamp-thunk inline-block">
          <Stamp tone={back ? "good" : "quiet"}>{back ? "Subscribed" : "Unsubscribed"}</Stamp>
        </span>
        {back ? (
          <>
            <p role="status" className="text-lg">You&apos;re subscribed to {name} again.</p>
            <p className="text-sm text-muted-foreground">Welcome back. Nothing changed: your next issue arrives as usual.</p>
          </>
        ) : (
          <>
            <p role="status" className="text-lg">You&apos;re unsubscribed from {name}.</p>
            <p className="text-sm text-muted-foreground">No more issues will come. Clicked by mistake? One click puts you back.</p>
            {state.error && <p role="alert" className="text-sm text-destructive">{state.error}</p>}
            <input type="hidden" name="do" value="resubscribe" />
            <Button type="submit" variant="outline" disabled={pending}>Resubscribe</Button>
          </>
        )}
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
