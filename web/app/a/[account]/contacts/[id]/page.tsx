import { Suspense } from "react";
import { ConfirmAction } from "@/components/confirm-action";
import { Loading } from "@/components/loading";
import { BlockedBadge, ContactStatusBadge } from "@/components/status-badge";
import { engine, type Schemas } from "@/lib/engine";
import { utc } from "@/lib/format";
import { deleteContact, unsubscribeContact } from "../actions";
import { EditContact } from "./edit-contact";

export const metadata = { title: "Contact" };

type Props = { params: Promise<{ account: string; id: string }> };

export default function ContactPage(props: Props) {
  return (
    <Suspense fallback={<Loading />}>
      <Detail {...props} />
    </Suspense>
  );
}

async function Detail({ params }: Props) {
  const { account, id } = await params;
  const c = await engine<Schemas["ContactView"]>(`/contacts/${id}`, { account, nextPath: `/a/${account}/contacts/${id}` });
  return (
    <div className="flex flex-col gap-6">
      <a href={`/a/${account}/contacts`} className="text-sm text-muted-foreground underline-offset-2 hover:underline">← Contacts</a>
      <div className="flex flex-wrap items-center gap-2">
        <h1 className="break-all text-2xl font-semibold">{c.email}</h1>
        <ContactStatusBadge status={c.status} />
        {c.blocked && <BlockedBadge />}
      </div>
      <dl className="grid max-w-xl grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">Source</dt><dd>{c.source}</dd>
        <dt className="text-muted-foreground">Added</dt><dd>{utc(c.created_at)}</dd>
        {c.subscribed_at && (<><dt className="text-muted-foreground">Subscribed at</dt><dd>{utc(c.subscribed_at)}</dd></>)}
        {c.unsubscribed_at && (<><dt className="text-muted-foreground">Unsubscribed at</dt><dd>{utc(c.unsubscribed_at)}</dd></>)}
      </dl>
      {c.blocked && <p className="max-w-prose text-sm text-muted-foreground">This address is on your suppression list, so nothing will be sent to it.</p>}
      <EditContact account={account} contact={c} />
      <div className="flex flex-wrap gap-2">
        {c.status !== "unsubscribed" && (
          <ConfirmAction
            trigger="Unsubscribe"
            title={`Unsubscribe ${c.email}?`}
            description="They stop getting the newsletter. Only they can subscribe again, through the signup form or their own unsubscribe link."
            confirm="Unsubscribe contact"
            action={unsubscribeContact.bind(null, account, c.id)}
          />
        )}
        <ConfirmAction
          trigger="Delete"
          title={`Delete ${c.email}?`}
          description="Erases the contact, their tags and fields for good. If they unsubscribed, a later import can add them back as subscribed; unsubscribe instead to keep their choice."
          confirm="Delete contact"
          action={deleteContact.bind(null, account, c.id)}
        />
      </div>
    </div>
  );
}
