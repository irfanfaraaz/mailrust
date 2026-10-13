import { Suspense } from "react";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { buttonVariants } from "@/components/ui/button";
import { engine, type Schemas } from "@/lib/engine";
import { CreateWebhook } from "./create-webhook";
import { deleteWebhook } from "./actions";

type Props = { params: Promise<{ account: string }> };

export default function Webhooks(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Webhooks</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
        <List {...props} />
      </Suspense>
    </div>
  );
}

async function List({ params }: Props) {
  const { account } = await params;
  const hooks = await engine<Schemas["WebhookList"]>("/webhooks", { account, nextPath: `/a/${account}/webhooks` });
  return (
    <>
      <CreateWebhook account={account} />
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead><tr><th className="text-left">URL</th><th className="text-left">Events</th><th className="text-left">Enabled</th><th className="text-right">Created</th><th /></tr></thead>
          <tbody>
            {hooks.data.map((h) => (
              <tr key={h.id}>
                <td className="max-w-64 truncate">{h.url}</td>
                <td>{h.events.join(", ")}</td>
                <td>{h.enabled ? "yes" : "no"}</td>
                <td className="text-right tabular-nums">{new Date(h.created_at).toLocaleString()}</td>
                <td className="text-right">
                  <Dialog>
                    <DialogTrigger className={buttonVariants({ variant: "outline", size: "sm" })}>Delete</DialogTrigger>
                    <DialogContent>
                      <DialogHeader>
                        <DialogTitle>Delete this webhook?</DialogTitle>
                        <DialogDescription>Mailrust stops sending events to {h.url}.</DialogDescription>
                      </DialogHeader>
                      <DialogFooter>
                        <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
                        <form action={deleteWebhook.bind(null, account, h.id)}>
                          <button type="submit" className={buttonVariants({ variant: "destructive" })}>Delete webhook</button>
                        </form>
                      </DialogFooter>
                    </DialogContent>
                  </Dialog>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {hooks.data.length === 0 && <p className="text-muted-foreground">No webhooks yet.</p>}
    </>
  );
}
