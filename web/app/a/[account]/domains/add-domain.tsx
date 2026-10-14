"use client";
import { useActionState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { addDomain, type AddDomainState } from "./actions";

export function AddDomain({ account }: { account: string }) {
  const [state, action, pending] = useActionState(addDomain.bind(null, account), {} as AddDomainState);
  return (
    <Card>
      <CardHeader>
        <h2 className="text-lg leading-snug font-semibold">New domain</h2>
      </CardHeader>
      <CardContent>
        <form action={action} className="flex flex-col gap-4 text-sm">
          <div className="flex flex-wrap gap-3">
            <label className="flex min-w-60 flex-col gap-1.5"><span className="label-caps text-muted-foreground">Domain</span>
              <Input id="domain-name" name="name" required placeholder="news.example.com" className="font-mono" />
            </label>
            <label className="flex min-w-60 flex-col gap-1.5"><span className="label-caps text-muted-foreground">Tracking subdomain (optional)</span>
              <Input name="tracking_subdomain" placeholder="track" className="font-mono" />
            </label>
          </div>
          <div className="flex flex-wrap gap-x-5">
            <label className="flex items-center gap-1.5 max-md:min-h-11"><input type="checkbox" name="open_tracking" className="accent-primary" />Track opens</label>
            <label className="flex items-center gap-1.5 max-md:min-h-11"><input type="checkbox" name="click_tracking" className="accent-primary" />Track clicks</label>
          </div>
          <Button type="submit" className="self-start" disabled={pending}>Add domain</Button>
          {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
        </form>
      </CardContent>
    </Card>
  );
}
