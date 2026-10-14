import { Suspense } from "react";
import { ActionForm } from "@/components/action-form";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { buttonVariants } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { CreateKey } from "./create-key";
import { deleteKey } from "./actions";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "API keys" };

type Props = { params: Promise<{ account: string }> };

export default function ApiKeys(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">API keys</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
        <Keys {...props} />
      </Suspense>
    </div>
  );
}

async function Keys({ params }: Props) {
  const { account } = await params;
  const keys = await engine<Schemas["KeyList"]>("/api-keys", { account, nextPath: `/a/${account}/api-keys` });
  return (
    <>
      <CreateKey account={account} />
      {keys.data.length === 0 ? (
        <EmptyState
          title="No API keys yet"
          hint="Your app sends email by calling the API with a key. Create one, then pass it as a Bearer token."
          action="Create your first key"
          href="#key-name"
        />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Key</TableHead>
              <TableHead>Access</TableHead>
              <TableHead>Last used</TableHead>
              <TableHead className="text-right">Created</TableHead>
              <TableHead><span className="sr-only">Actions</span></TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {keys.data.map((k) => (
              <TableRow key={k.id}>
                <TableCell className="font-medium">{k.name}</TableCell>
                <TableCell className="font-mono">{k.prefix}…</TableCell>
                <TableCell>{k.access}</TableCell>
                <TableCell className="tabular-nums text-muted-foreground">{k.last_used_at ? <LocalTime iso={k.last_used_at} /> : "never"}</TableCell>
                <TableCell className="text-right tabular-nums text-muted-foreground"><LocalTime iso={k.created_at} /></TableCell>
                <TableCell className="text-right">
                  <Dialog>
                    <DialogTrigger className={buttonVariants({ variant: "outline", size: "sm" })}>Delete</DialogTrigger>
                    <DialogContent>
                      <DialogHeader>
                        <DialogTitle>Delete this key?</DialogTitle>
                        <DialogDescription>Delete this key? Apps using it stop working at once.</DialogDescription>
                      </DialogHeader>
                      <DialogFooter>
                        <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
                        <ActionForm action={deleteKey.bind(null, account, k.id)} done="Key deleted">
                          <button type="submit" className={buttonVariants({ variant: "destructive" })}>Delete key</button>
                        </ActionForm>
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
