"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { confirmSubscription, type ConfirmState } from "./actions";

export function ConfirmForm({ token }: { token: string }) {
  const [state, action, pending] = useActionState<ConfirmState>(confirmSubscription.bind(null, token), {});
  if (state.name) return <p role="status" className="text-lg">You&apos;re subscribed to {state.name}.</p>;
  return (
    <form action={action} className="flex flex-col gap-3">
      <p>Confirm that you want to get this newsletter.</p>
      {state.error && <p role="alert" className="text-sm text-destructive">{state.error}</p>}
      <Button type="submit" disabled={pending}>Confirm</Button>
    </form>
  );
}
