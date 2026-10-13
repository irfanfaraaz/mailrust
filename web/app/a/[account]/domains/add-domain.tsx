"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { addDomain, type AddDomainState } from "./actions";

export function AddDomain({ account }: { account: string }) {
  const [state, action, pending] = useActionState(addDomain.bind(null, account), {} as AddDomainState);
  return (
    <form action={action} className="flex flex-col gap-3 text-sm">
      <label className="flex flex-col gap-1">Domain
        <Input name="name" required placeholder="news.example.com" />
      </label>
      <label className="flex flex-col gap-1">Tracking subdomain (optional)
        <Input name="tracking_subdomain" placeholder="track" />
      </label>
      <label className="flex items-center gap-1.5"><input type="checkbox" name="open_tracking" />Track opens</label>
      <label className="flex items-center gap-1.5"><input type="checkbox" name="click_tracking" />Track clicks</label>
      <Button type="submit" className="self-start" disabled={pending}>Add domain</Button>
      {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
    </form>
  );
}
