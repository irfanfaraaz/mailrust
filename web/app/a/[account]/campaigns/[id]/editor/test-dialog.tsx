"use client";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { buttonVariants } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { sendTest } from "../actions";

export function TestDialog({ account, id, me, beforeSend }: { account: string; id: string; me: string; beforeSend: () => Promise<void> }) {
  const [state, setState] = useState<{ error?: string; done?: boolean; pending?: boolean }>({});
  return (
    <Dialog onOpenChange={() => setState({})}>
      <DialogTrigger className={buttonVariants({ variant: "outline" }) + " w-full"}>Send test</DialogTrigger>
      <DialogContent>
        <DialogHeader><DialogTitle>Send a test</DialogTitle></DialogHeader>
        <form
          className="flex flex-col gap-3 text-sm"
          onSubmit={async (e) => {
            e.preventDefault();
            const to = String(new FormData(e.currentTarget).get("to") ?? "").split(",").map((s) => s.trim()).filter(Boolean);
            setState({ pending: true });
            await beforeSend();
            const r = await sendTest(account, id, to);
            setState(r.error ? { error: r.error } : { done: true });
            if (!r.error) toast.success("Test sent");
          }}
        >
          <label className="flex flex-col gap-1">Send to
            <Input name="to" defaultValue={me} />
            <span className="text-muted-foreground">Up to 5 team members, comma-separated. The subject starts with [Test].</span>
          </label>
          {state.error && <p role="alert" className="text-destructive">{state.error}</p>}
          {state.done && (
            <div className="flex flex-col items-center gap-1 text-center">
              {/* The envelope drops through a slot: the test is on its way. */}
              <div aria-hidden className="mail-slot">
                <svg viewBox="0 0 44 32" width="44" height="32"><rect x="2" y="2" width="40" height="28" rx="3" /><path d="M2 5 L22 19 L42 5" /></svg>
                <span />
              </div>
              <p role="status">Test sent. It arrives in a minute.</p>
            </div>
          )}
          <Button type="submit" disabled={state.pending} className="self-start">Send test email</Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
