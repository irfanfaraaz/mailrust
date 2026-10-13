import { Suspense } from "react";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { buttonVariants } from "@/components/ui/button";
import { engine, type Schemas } from "@/lib/engine";
import { CreateKey } from "./create-key";
import { deleteKey } from "./actions";

type Props = { params: Promise<{ account: string }> };

export default function ApiKeys(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">API keys</h1>
      {/* params and the engine call are request data: read them inside Suspense (cacheComponents). */}
      <Suspense>
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
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead><tr><th className="text-left">Name</th><th className="text-left">Key</th><th className="text-left">Access</th><th className="text-left">Last used</th><th className="text-right">Created</th><th /></tr></thead>
          <tbody>
            {keys.data.map((k) => (
              <tr key={k.id}>
                <td>{k.name}</td>
                <td className="font-mono">{k.prefix}…</td>
                <td>{k.access}</td>
                <td>{k.last_used_at ? new Date(k.last_used_at).toLocaleString() : "never"}</td>
                <td className="text-right tabular-nums">{new Date(k.created_at).toLocaleString()}</td>
                <td className="text-right">
                  <Dialog>
                    <DialogTrigger className={buttonVariants({ variant: "outline", size: "sm" })}>Delete</DialogTrigger>
                    <DialogContent>
                      <DialogHeader>
                        <DialogTitle>Delete this key?</DialogTitle>
                        <DialogDescription>Delete this key? Apps using it stop working at once.</DialogDescription>
                      </DialogHeader>
                      <DialogFooter>
                        <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
                        <form action={deleteKey.bind(null, account, k.id)}>
                          <button type="submit" className={buttonVariants({ variant: "destructive" })}>Delete key</button>
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
      {keys.data.length === 0 && <p className="text-muted-foreground">No API keys yet.</p>}
    </>
  );
}
