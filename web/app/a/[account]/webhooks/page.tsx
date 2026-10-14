import { Suspense } from "react";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { buttonVariants } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { CreateWebhook } from "./create-webhook";
import { deleteWebhook } from "./actions";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Webhooks" };

type Props = { params: Promise<{ account: string }> };

export default function Webhooks(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Webhooks</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
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
      {hooks.data.length === 0 ? (
        <EmptyState
          title="No webhooks yet"
          hint="Mailrust can POST each delivery, bounce or complaint to your app as it happens, so you do not have to poll."
          action="Add your first webhook"
          href="#webhook-url"
        />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>URL</TableHead>
              <TableHead>Events</TableHead>
              <TableHead>Enabled</TableHead>
              <TableHead className="text-right">Created</TableHead>
              <TableHead><span className="sr-only">Actions</span></TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {hooks.data.map((h) => (
              <TableRow key={h.id}>
                <TableCell className="max-w-64 truncate font-medium">{h.url}</TableCell>
                <TableCell className="min-w-64 whitespace-normal text-muted-foreground">{h.events.join(", ")}</TableCell>
                <TableCell>{h.enabled ? "yes" : "no"}</TableCell>
                <TableCell className="text-right tabular-nums text-muted-foreground"><LocalTime iso={h.created_at} /></TableCell>
                <TableCell className="text-right">
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
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </>
  );
}
