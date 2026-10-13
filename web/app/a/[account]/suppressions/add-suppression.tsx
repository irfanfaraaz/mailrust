"use client";
import { useActionState, useEffect } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { addSuppression, type AddSuppressionState } from "./actions";

const none: AddSuppressionState = {};

export function AddSuppression({ account }: { account: string }) {
  const [state, action, pending] = useActionState(addSuppression.bind(null, account), none);
  // Each result is a new object, so this fires once per successful submit.
  useEffect(() => {
    if (state !== none && !state.error) toast.success("Address blocked");
  }, [state]);
  return (
    <form action={action} className="flex flex-wrap items-end gap-3 text-sm">
      <label className="flex flex-col gap-1">Address
        <Input name="address" type="email" required />
      </label>
      <Button type="submit" disabled={pending}>Block address</Button>
      {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
    </form>
  );
}
