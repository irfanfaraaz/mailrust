"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { confirmLogin, type FormState } from "../actions";

export function ConfirmForm({ token, next }: { token: string; next: string }) {
  const [state, action, pending] = useActionState<FormState, FormData>(confirmLogin, {});
  return (
    <form action={action} className="flex flex-col gap-3">
      <input type="hidden" name="token" value={token} />
      <input type="hidden" name="next" value={next} />
      {state.error && <p role="alert" className="text-sm text-destructive">{state.error} <a className="underline" href="/login">Get a new link</a></p>}
      <Button type="submit" disabled={pending}>Log in</Button>
    </form>
  );
}
