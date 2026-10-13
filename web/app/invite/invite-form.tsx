"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import type { FormState } from "../login/actions";
import { acceptInvite } from "./actions";

export function InviteForm({ token }: { token: string }) {
  const [state, action, pending] = useActionState<FormState, FormData>(acceptInvite, {});
  return (
    <form action={action} className="flex flex-col gap-3">
      <input type="hidden" name="token" value={token} />
      {state.error && <p role="alert" className="text-sm text-destructive">{state.error}</p>}
      <Button type="submit" disabled={pending}>Accept invite</Button>
    </form>
  );
}
