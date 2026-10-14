"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { confirmSubscription, type ConfirmState } from "./actions";

const heading = "text-2xl font-semibold";

export function ConfirmForm({ token, name, slug }: { token: string; name: string | null; slug: string | null }) {
  const [state, action, pending] = useActionState<ConfirmState>(confirmSubscription.bind(null, token), {});
  if (state.name) {
    return (
      <>
        <h1 className={heading}>Subscription confirmed</h1>
        <p role="status" className="text-lg text-good">You&apos;re subscribed to {state.name}.</p>
        <p className="text-sm text-muted-foreground">The next issue comes to this inbox. Every issue has an unsubscribe link.</p>
      </>
    );
  }
  if (state.expired) {
    return (
      <>
        <h1 className={heading}>This link no longer works</h1>
        <div role="alert" className="flex flex-col gap-2">
          <p>It was already used, or it is older than 7 days.</p>
          <p className="text-sm text-muted-foreground">
            If you confirmed before, you are already subscribed. If not, sign up again on the newsletter&apos;s signup form to get a new link.
          </p>
          {slug && <a className="text-primary underline underline-offset-2" href={`/join/${encodeURIComponent(slug)}`}>Sign up again</a>}
        </div>
      </>
    );
  }
  return (
    <>
      <h1 className={heading}>{name ? `Confirm your subscription to ${name}` : "Confirm your subscription"}</h1>
      <form action={action} className="flex flex-col gap-3">
        <p>Press the button to start getting this newsletter.</p>
        <Button type="submit" disabled={pending} className="self-start">{pending ? "Confirming…" : "Confirm"}</Button>
      </form>
    </>
  );
}
