import Link from "next/link";
import { Suspense } from "react";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { NativeSelect } from "@/components/native-select";
import { BlockedBadge, ContactStatusBadge } from "@/components/status-badge";
import { PageHeader } from "@/components/page-header";
import { Button, buttonVariants } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { contactCount } from "@/lib/format";
import { AddContact } from "./add-contact";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Contacts" };

type SP = { q?: string; status?: string; tag?: string; after?: string };
type Props = { params: Promise<{ account: string }>; searchParams: Promise<SP> };

const STATUSES: Schemas["ContactStatus"][] = ["subscribed", "pending", "unsubscribed"];
const STATUS_LABEL: Record<Schemas["ContactStatus"], string> = { subscribed: "Subscribed", pending: "Pending (not confirmed)", unsubscribed: "Unsubscribed" };

export default function Contacts(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      {/* params, searchParams and the engine calls are request data: read them inside Suspense (cacheComponents). */}
      <PageHeader
        title="Contacts"
        description="Everyone who can get your email, and where they stand."
        actions={
          <Suspense fallback={null}>
            <Actions params={props.params} />
          </Suspense>
        }
      />
      <Suspense fallback={<Loading />}>
        <List {...props} />
      </Suspense>
    </div>
  );
}

async function Actions({ params }: Pick<Props, "params">) {
  const { account } = await params;
  return (
    <>
      <Link href={`/a/${account}/contacts/import`} className={buttonVariants({ variant: "outline", size: "sm" })}>Import</Link>
      {/* A file download from a route handler: a plain link, not client navigation. */}
      <a href={`/a/${account}/contacts/export`} download className={buttonVariants({ variant: "outline", size: "sm" })}>Export CSV</a>
    </>
  );
}

async function List({ params, searchParams }: Props) {
  const { account } = await params;
  const sp = await searchParams;
  const filters = Object.fromEntries((["q", "status", "tag"] as const).filter((k) => sp[k]).map((k) => [k, sp[k]!]));
  const qs = new URLSearchParams({ limit: "50", ...filters, ...(sp.after ? { after: sp.after } : {}) });
  const base = `/a/${account}/contacts`;
  const [list, tags] = await Promise.all([
    engine<Schemas["ContactList"]>(`/contacts?${qs}`, { account, nextPath: base }),
    engine<Schemas["TagList"]>("/tags", { account, nextPath: base }),
  ]);
  const older = new URLSearchParams({ ...filters, after: list.next_cursor ?? "" });
  const hasFilters = Object.keys(filters).length > 0;
  const n = list.data.length;
  const count = contactCount(list.total, n, Boolean(list.next_cursor || sp.after));
  const clear = <Link className="underline underline-offset-2" href={base}>Clear filters</Link>;
  return (
    <>
      <section aria-labelledby="add-contact-title">
        <Card>
          <CardHeader>
            <h2 id="add-contact-title" className="text-lg leading-snug font-semibold">Add a contact</h2>
            <p className="text-sm text-muted-foreground">Subscribed at once, with no confirm email: only for people who already agreed.</p>
          </CardHeader>
          <CardContent>
            <AddContact account={account} />
          </CardContent>
        </Card>
      </section>
      {n === 0 && !hasFilters && !sp.after ? (
        <EmptyState
          title="No contacts yet"
          hint="Add people with the form above, import a CSV from your old tool, or turn on the signup form so readers can join."
          action="Import a CSV"
          href={`${base}/import`}
        />
      ) : (
        <section aria-label="Contact list" className="flex flex-col gap-4">
          {/* A plain GET form: filters live in the URL and need no JS. */}
          <form method="get" className="flex flex-wrap items-end gap-3 text-sm">
            <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Search</span>
              <Input name="q" type="search" placeholder="Email or name" defaultValue={sp.q ?? ""} />
            </label>
            <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Status</span>
              <NativeSelect name="status" defaultValue={sp.status ?? ""}>
                <option value="">Any</option>
                {STATUSES.map((s) => <option key={s} value={s}>{STATUS_LABEL[s]}</option>)}
              </NativeSelect>
            </label>
            <label className="flex flex-col gap-1.5"><span className="label-caps text-muted-foreground">Tag</span>
              <NativeSelect name="tag" defaultValue={sp.tag ?? ""}>
                <option value="">Any</option>
                {tags.data.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
              </NativeSelect>
            </label>
            <Button type="submit" variant="outline">Filter</Button>
            {hasFilters && <span className="pb-2">{clear}</span>}
          </form>
          {n === 0 ? (
            <p className="sketch sketch-lg bg-card p-6 text-sm text-muted-foreground">
              No contacts match these filters. {clear}
            </p>
          ) : (
            <>
              <p className="text-sm text-muted-foreground tabular-nums">{count}</p>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Email</TableHead>
                    <TableHead>Name</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Tags</TableHead>
                    <TableHead className="text-right">Added</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {list.data.map((c) => (
                    <TableRow key={c.id}>
                      <TableCell className="font-medium">
                        <Link className="underline-offset-2 hover:underline" href={`${base}/${c.id}`}>{c.email}</Link>
                      </TableCell>
                      <TableCell>{[c.first_name, c.last_name].filter(Boolean).join(" ")}</TableCell>
                      <TableCell>
                        <span className="flex flex-wrap gap-1.5">
                          <ContactStatusBadge status={c.status} />
                          {c.blocked && <BlockedBadge />}
                        </span>
                      </TableCell>
                      <TableCell className="text-muted-foreground">{c.tags.map((t) => t.name).join(", ")}</TableCell>
                      <TableCell className="text-right tabular-nums text-muted-foreground"><LocalTime iso={c.created_at} /></TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
              {list.next_cursor && <Link className="self-start underline underline-offset-2" href={`?${older}`}>Older →</Link>}
            </>
          )}
        </section>
      )}
    </>
  );
}
