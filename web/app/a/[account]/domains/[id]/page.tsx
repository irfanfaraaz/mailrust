import { Suspense } from "react";
import { ActionForm } from "@/components/action-form";
import { ConfirmAction } from "@/components/confirm-action";
import { BackLink } from "@/components/back-link";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { DomainStatusBadge } from "@/components/status-badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { deleteDomain, updateTracking, verifyDomain } from "./actions";
import { CopyButton } from "./copy-button";

export const metadata = { title: "Domain" };

type Props = { params: Promise<{ account: string; id: string }> };

// What the last check found, and what to do next.
const checkResult: Record<Schemas["DomainStatus"], { text: string; className: string }> = {
  verified: { text: "All records found. You can send from this domain.", className: "text-good" },
  pending: {
    text: "Not verified yet: Mailrust has not found every record. DNS changes can take from a few minutes to a few hours to show up, so check again later.",
    className: "text-muted-foreground",
  },
  failed: {
    text: "Verification failed: at least one record is missing or has a different value. Compare each record at your DNS provider with the table above, fix it, then check again.",
    className: "text-destructive",
  },
};

export default function DomainPage(props: Props) {
  return (
    <Suspense fallback={<Loading rows={6} />}>
      <Detail {...props} />
    </Suspense>
  );
}

async function Detail({ params }: Props) {
  const { account, id } = await params;
  const d = await engine<Schemas["Domain"]>(`/domains/${id}`, { account, nextPath: `/a/${account}/domains/${id}` });
  const result = checkResult[d.status];
  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={<span className="font-mono break-all text-2xl">{d.name}</span>} actions={<DomainStatusBadge status={d.status} />}>
        <BackLink href={`/a/${account}/domains`}>Domains</BackLink>
      </PageHeader>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">DNS records</h2>
        <ol className="flex max-w-prose list-decimal flex-col gap-1 pl-5 text-sm text-muted-foreground">
          <li>Open the DNS settings for {d.name} at your domain registrar or DNS host.</li>
          <li>Create each record below with the exact type, name and value. Use Copy to avoid typos.</li>
          <li>Press Check now to see whether Mailrust can find them.</li>
        </ol>
        <Table aria-label="DNS records">
          <TableHeader>
            <TableRow>
              <TableHead>Type</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Value</TableHead>
              <TableHead>Purpose</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {d.dns_records.map((r) => (
              <TableRow key={`${r.record_type}-${r.name}`}>
                <TableCell className="font-mono text-[0.8125rem] font-bold">{r.record_type}</TableCell>
                <TableCell className="whitespace-normal break-all font-mono text-[0.8125rem]">{r.name}</TableCell>
                <TableCell className="whitespace-normal">
                  <div className="flex items-center gap-2">
                    <code className="max-w-md break-all font-mono text-[0.8125rem]">{r.value}</code>
                    <CopyButton value={r.value} label={`${r.record_type} value for ${r.name}`} />
                  </div>
                </TableCell>
                <TableCell className="text-muted-foreground">{r.purpose}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        <div className="flex flex-col items-start gap-2">
          <ActionForm action={verifyDomain.bind(null, account, id)} done="DNS checked">
            <Button type="submit" variant="outline">Check now</Button>
          </ActionForm>
          <p role="status" className={`max-w-prose text-sm ${result.className}`}>{result.text}</p>
        </div>
      </section>

      <section className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">Tracking</h2>
        <Card>
          <CardContent>
            <ActionForm action={updateTracking.bind(null, account, id)} done="Tracking saved" className="flex flex-col gap-2 text-sm">
              <label className="flex items-center gap-1.5 max-md:min-h-11"><input type="checkbox" name="open_tracking" defaultChecked={d.open_tracking} className="accent-primary" />Track opens</label>
              <label className="flex items-center gap-1.5 max-md:min-h-11"><input type="checkbox" name="click_tracking" defaultChecked={d.click_tracking} className="accent-primary" />Track clicks</label>
              <Button type="submit" className="mt-2 self-start">Save tracking</Button>
            </ActionForm>
          </CardContent>
        </Card>
      </section>

      <Card>
        <CardHeader>
          <h2 className="text-lg leading-snug font-semibold text-destructive">Remove domain</h2>
        </CardHeader>
        <CardContent className="flex flex-col items-start gap-3">
          <p className="text-sm text-muted-foreground">Emails from this domain will be refused.</p>
          <ConfirmAction
            trigger="Remove domain"
            triggerClassName={buttonVariants({ variant: "destructive" })}
            title={`Remove ${d.name}?`}
            description="Emails from this domain will be refused."
            confirm="Remove domain"
            done="Domain removed"
            action={deleteDomain.bind(null, account, id)}
          />
        </CardContent>
      </Card>
    </div>
  );
}
