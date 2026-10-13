import Link from "next/link";
import { Suspense } from "react";
import { BackLink } from "@/components/back-link";
import { ConfirmAction } from "@/components/confirm-action";
import { Loading } from "@/components/loading";
import { BlockedBadge, ContactStatusBadge, PENDING_HINT } from "@/components/status-badge";
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
      <BackLink href={`/a/${account}/contacts`}>Contacts</BackLink>
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
      {c.status === "pending" && <p className="max-w-prose text-sm text-muted-foreground">Pending: {PENDING_HINT.charAt(0).toLowerCase() + PENDING_HINT.slice(1)}</p>}
      {c.blocked && (
        <p className="max-w-prose text-sm text-muted-foreground">
          This address is on your{" "}
          <Link className="text-foreground underline underline-offset-2" href={`/a/${account}/suppressions`}>suppression list</Link>, so nothing will be sent to it.
        </p>
      )}
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
