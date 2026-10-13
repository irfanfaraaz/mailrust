import { Suspense } from "react";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Button, buttonVariants } from "@/components/ui/button";
import { engine, type Schemas } from "@/lib/engine";
import { StatusBadge } from "../page";
import { deleteDomain, updateTracking, verifyDomain } from "./actions";
import { CopyButton } from "./copy-button";

type Props = { params: Promise<{ account: string; id: string }> };

export default function DomainPage(props: Props) {
  return (
    <Suspense>
      <Detail {...props} />
    </Suspense>
  );
}

async function Detail({ params }: Props) {
  const { account, id } = await params;
  const d = await engine<Schemas["Domain"]>(`/domains/${id}`, { account, nextPath: `/a/${account}/domains/${id}` });
  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-semibold">{d.name}</h1>
        <StatusBadge status={d.status} />
      </div>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-medium">DNS records</h2>
        <div className="overflow-x-auto">
          <table aria-label="DNS records" className="w-full text-sm">
            <thead><tr><th className="text-left">Type</th><th className="text-left">Name</th><th className="text-left">Value</th><th className="text-left">Purpose</th></tr></thead>
            <tbody>
              {d.dns_records.map((r) => (
                <tr key={`${r.record_type}-${r.name}`}>
                  <td>{r.record_type}</td>
                  <td className="break-all">{r.name}</td>
                  <td>
                    <div className="flex items-center gap-2">
                      <code className="break-all">{r.value}</code>
                      <CopyButton value={r.value} label={`${r.record_type} value for ${r.name}`} />
                    </div>
                  </td>
                  <td>{r.purpose}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <form action={verifyDomain.bind(null, account, id)}>
          <Button type="submit" variant="secondary">Check now</Button>
        </form>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-medium">Tracking</h2>
        <form action={updateTracking.bind(null, account, id)} className="flex flex-col gap-2 text-sm">
          <label className="flex items-center gap-1.5"><input type="checkbox" name="open_tracking" defaultChecked={d.open_tracking} />Track opens</label>
          <label className="flex items-center gap-1.5"><input type="checkbox" name="click_tracking" defaultChecked={d.click_tracking} />Track clicks</label>
          <Button type="submit" className="self-start">Save tracking</Button>
        </form>
      </section>

      <Dialog>
        <DialogTrigger className={buttonVariants({ variant: "outline" }) + " self-start"}>Remove domain</DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Remove {d.name}?</DialogTitle>
            <DialogDescription>Emails from this domain will be refused.</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
            <form action={deleteDomain.bind(null, account, id)}>
              <button type="submit" className={buttonVariants({ variant: "destructive" })}>Remove domain</button>
            </form>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
