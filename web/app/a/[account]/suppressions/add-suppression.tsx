"use client";
import { useActionState, useEffect } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
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
    <Card>
      <CardHeader>
        <h2 className="text-lg leading-snug font-semibold">Block an address</h2>
      </CardHeader>
      <CardContent>
        <form action={action} className="flex flex-wrap items-end gap-3 text-sm">
          <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Address</span>
            <Input name="address" type="email" required className="font-mono" />
          </label>
          <Button type="submit" disabled={pending}>Block address</Button>
          {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
        </form>
      </CardContent>
    </Card>
  );
}
