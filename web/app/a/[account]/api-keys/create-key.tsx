"use client";
import { useActionState } from "react";
import { CopyOnce } from "@/components/copy-once";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { createKey, type CreateKeyState } from "./actions";

const select = "h-8 rounded-lg border border-input bg-transparent px-2 text-sm";

export function CreateKey({ account }: { account: string }) {
  const [state, action, pending] = useActionState(createKey.bind(null, account), {} as CreateKeyState);
  return (
    <div className="flex flex-col gap-4">
      <form action={action} className="flex flex-wrap items-end gap-3 text-sm">
        <label className="flex flex-col gap-1">Name
          <Input name="name" required maxLength={100} />
        </label>
        <label className="flex flex-col gap-1">Access
          <select name="access" className={select} defaultValue="full">
            <option value="full">full</option>
            <option value="sending">sending</option>
          </select>
        </label>
        <Button type="submit" disabled={pending}>Create key</Button>
      </form>
      {state.error && <p role="alert" className="text-sm text-destructive">{state.error}</p>}
      {state.created && <CopyOnce label="Your new API key" value={state.created.token} />}
    </div>
  );
}
