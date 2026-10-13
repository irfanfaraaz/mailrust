import Link from "next/link";
import { Suspense } from "react";
import { EmptyState } from "@/components/empty-state";
import { Loading } from "@/components/loading";
import { NativeSelect } from "@/components/native-select";
import { BlockedBadge, ContactStatusBadge } from "@/components/status-badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { utc } from "@/lib/format";
import { AddContact } from "./add-contact";

export const metadata = { title: "Contacts" };

type SP = { q?: string; status?: string; tag?: string; after?: string };
type Props = { params: Promise<{ account: string }>; searchParams: Promise<SP> };

const STATUSES: Schemas["ContactStatus"][] = ["subscribed", "pending", "unsubscribed"];

export default function Contacts(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Contacts</h1>
      {/* params, searchParams and the engine calls are request data: read them inside Suspense (cacheComponents). */}
      <Suspense fallback={<Loading />}>
        <List {...props} />
      </Suspense>
    </div>
  );
}

async function List({ params, searchParams }: Props) {
  const { account } = await params;
  const sp = await searchParams;
  const filters = Object.fromEntries((["q", "status", "tag"] as const).filter((k) => sp[k]).map((k) => [k, sp[k]!]));
  const qs = new URLSearchParams({ limit: "50", ...filters, ...(sp.after ? { after: sp.after } : {}) });
  const nextPath = `/a/${account}/contacts`;
  const [list, tags] = await Promise.all([
    engine<Schemas["ContactList"]>(`/contacts?${qs}`, { account, nextPath }),
    engine<Schemas["TagList"]>("/tags", { account, nextPath }),
  ]);
  const older = new URLSearchParams({ ...filters, after: list.next_cursor ?? "" });
  const filtered = Object.keys(filters).length > 0 || Boolean(sp.after);
  return (
    <>
      <div className="flex flex-wrap gap-2">
        <a href={`/a/${account}/contacts/import`} className={buttonVariants({ variant: "outline", size: "sm" })}>Import</a>
        <a href={`/a/${account}/contacts/export`} download className={buttonVariants({ variant: "outline", size: "sm" })}>Export CSV</a>
      </div>
      <AddContact account={account} />
      {list.data.length === 0 && !filtered ? (
        <EmptyState
          title="No contacts yet"
          hint="Add people one by one, import a CSV from your old tool, or turn on the signup form so readers can join."
          action="Add your first contact"
          href="#contact-email"
        />
      ) : (
        <>
          {/* A plain GET form: filters live in the URL and need no JS. */}
          <form method="get" className="flex flex-wrap items-end gap-3 text-sm">
            <label className="flex flex-col gap-1">Search
              <Input name="q" type="search" placeholder="Email or name" defaultValue={sp.q ?? ""} />
            </label>
            <label className="flex flex-col gap-1">Status
              <NativeSelect name="status" defaultValue={sp.status ?? ""}>
                <option value="">Any</option>
                {STATUSES.map((s) => <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>)}
              </NativeSelect>
            </label>
            <label className="flex flex-col gap-1">Tag
              <NativeSelect name="tag" defaultValue={sp.tag ?? ""}>
                <option value="">Any</option>
                {tags.data.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
              </NativeSelect>
            </label>
            <Button type="submit" variant="outline">Filter</Button>
          </form>
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
                    <Link className="underline-offset-2 hover:underline" href={`/a/${account}/contacts/${c.id}`}>{c.email}</Link>
                  </TableCell>
                  <TableCell>{[c.first_name, c.last_name].filter(Boolean).join(" ")}</TableCell>
                  <TableCell>
                    <span className="flex gap-1">
                      <ContactStatusBadge status={c.status} />
                      {c.blocked && <BlockedBadge />}
                    </span>
                  </TableCell>
                  <TableCell className="text-muted-foreground">{c.tags.map((t) => t.name).join(", ")}</TableCell>
                  <TableCell className="text-right tabular-nums text-muted-foreground">{utc(c.created_at)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {list.data.length === 0 && <p className="text-muted-foreground">No contacts match these filters.</p>}
          {list.next_cursor && <Link className="self-start underline underline-offset-2" href={`?${older}`}>Older →</Link>}
        </>
      )}
    </>
  );
}
