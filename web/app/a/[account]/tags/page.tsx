import Link from "next/link";
import { Suspense } from "react";
import { ConfirmAction } from "@/components/confirm-action";
import { Loading } from "@/components/loading";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { engine, type Schemas } from "@/lib/engine";
import { deleteTag } from "./actions";
import { AddTag, RenameTag } from "./tag-forms";

export const metadata = { title: "Tags" };

type Props = { params: Promise<{ account: string }> };

export default function Tags(props: Props) {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">Tags</h1>
      <p className="max-w-prose text-sm text-muted-foreground">Tags group contacts. Add them by hand, from a CSV column, or to every row of an import.</p>
      <Suspense fallback={<Loading />}>
        <List {...props} />
      </Suspense>
    </div>
  );
}

async function List({ params }: Props) {
  const { account } = await params;
  const tags = await engine<Schemas["TagList"]>("/tags", { account, nextPath: `/a/${account}/tags` });
  return (
    <>
      <AddTag account={account} />
      {tags.data.length === 0 ? (
        <p className="text-muted-foreground">No tags yet.</p>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead className="text-right">Contacts</TableHead>
              <TableHead><span className="sr-only">Actions</span></TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {tags.data.map((t) => (
              <TableRow key={t.id}>
                <TableCell>
                  <span className="sr-only">{t.name}</span>
                  <RenameTag account={account} id={t.id} name={t.name} />
                </TableCell>
                <TableCell className="text-right tabular-nums">
                  <Link className="underline underline-offset-2" href={`/a/${account}/contacts?tag=${t.id}`}>{t.contacts}</Link>
                </TableCell>
                <TableCell className="text-right">
                  <ConfirmAction
                    trigger="Delete"
                    title={`Delete the tag ${t.name}?`}
                    description="The tag comes off every contact. The contacts stay."
                    confirm="Delete tag"
                    action={deleteTag.bind(null, account, t.id)}
                  />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </>
  );
}
