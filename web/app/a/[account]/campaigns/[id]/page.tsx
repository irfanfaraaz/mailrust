import { Suspense } from "react";
import { BackLink } from "@/components/back-link";
import { ConfirmAction } from "@/components/confirm-action";
import { Loading } from "@/components/loading";
import { PageHeader } from "@/components/page-header";
import { CampaignStatusBadge } from "@/components/status-badge";
import { currentAccount } from "@/lib/account";
import { engine, type Schemas } from "@/lib/engine";
import { deleteDraft } from "./actions";
import { CampaignEditor } from "./editor/campaign-editor";
import { Report } from "./report";
import { LocalTime } from "@/components/local-time";

export const metadata = { title: "Campaign" };

type Props = { params: Promise<{ account: string; id: string }> };

export default function CampaignPage(props: Props) {
  return (
    <div className="flex flex-col gap-8">
      <Suspense fallback={<Loading />}>
        <Body {...props} />
      </Suspense>
    </div>
  );
}

async function Body({ params }: Props) {
  const { account, id } = await params;
  const nextPath = `/a/${account}/campaigns/${id}`;
  const [c, { me }, tags, newsletter] = await Promise.all([
    engine<Schemas["Campaign"]>(`/campaigns/${id}`, { account, nextPath }),
    currentAccount(account, nextPath),
    engine<Schemas["TagList"]>("/tags", { account, nextPath }),
    engine<Schemas["NewsletterSettings"]>("/newsletter", { account, nextPath }),
  ]);
  const editable = c.status === "draft" || c.status === "scheduled";
  const report = editable ? null : await engine<Schemas["CampaignReport"]>(`/campaigns/${id}/report`, { account, nextPath });
  const archiveUrl = c.status === "sent" && !c.archive_hidden && newsletter.public_slug && c.archive_slug ? `/p/${newsletter.public_slug}/${c.archive_slug}` : null;
  return (
    <>
      <PageHeader
        title={c.subject || "Untitled draft"}
        actions={
          <>
            <CampaignStatusBadge status={c.status} />
            {c.status === "draft" && (
              <ConfirmAction
              trigger="Delete draft"
              title="Delete this draft?"
              description="The draft is erased for good. This cannot be undone."
              confirm="Delete draft"
              dismiss="Keep it"
              done="Draft deleted"
              action={deleteDraft.bind(null, account, c.id)}
              />
            )}
          </>
        }
      >
        <BackLink href={`/a/${account}/campaigns`}>Campaigns</BackLink>
      </PageHeader>
      {c.status === "scheduled" && c.send_at && <p role="status" className="sketch bg-card px-4 py-3 text-sm">Scheduled for <LocalTime iso={c.send_at} />. Editing it puts it back to draft.</p>}
      {c.status === "draft" && c.paused_reason && <p role="alert" className="sketch bg-card px-4 py-3 text-sm text-destructive">Not sent: {c.paused_reason}</p>}
      {editable ? (
        <CampaignEditor account={account} campaign={c} me={me.user.email} tags={tags.data} />
      ) : (
        <Report account={account} campaign={c} report={report!} archiveUrl={archiveUrl} />
      )}
    </>
  );
}
