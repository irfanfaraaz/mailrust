"use client";
import { useActionState } from "react";
import { CopyOnce } from "@/components/copy-once";
import { NativeSelect } from "@/components/native-select";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { createKey, type CreateKeyState } from "./actions";

export function CreateKey({ account }: { account: string }) {
  const [state, action, pending] = useActionState(createKey.bind(null, account), {} as CreateKeyState);
  return (
    <div className="flex flex-col gap-5">
      {state.created && <CopyOnce label="Your new API key" value={state.created.token} />}
      <Card>
        <CardHeader>
          <h2 className="text-lg leading-snug font-semibold">New key</h2>
        </CardHeader>
        <CardContent>
          <form action={action} className="flex flex-wrap items-end gap-3 text-sm">
            <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Name</span>
              <Input id="key-name" name="name" required maxLength={100} />
            </label>
            <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Access</span>
              <NativeSelect name="access" defaultValue="full">
                <option value="full">full</option>
                <option value="sending">sending</option>
              </NativeSelect>
            </label>
            <Button type="submit" disabled={pending}>Create key</Button>
          </form>
          {state.error && <p role="alert" className="mt-3 text-sm text-destructive">{state.error}</p>}
        </CardContent>
      </Card>
    </div>
  );
}
