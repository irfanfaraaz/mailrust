import { Suspense } from "react";
import { BackLink } from "@/components/back-link";
import { Loading } from "@/components/loading";
import { CampaignStatusBadge } from "@/components/status-badge";
import { currentAccount } from "@/lib/account";
import { engine, type Schemas } from "@/lib/engine";
import { CampaignEditor } from "./editor/campaign-editor";

export const metadata = { title: "Campaign" };

type Props = { params: Promise<{ account: string; id: string }> };

export default function CampaignPage(props: Props) {
  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <Suspense fallback={<Loading />}>
        <Body {...props} />
      </Suspense>
    </div>
  );
}

async function Body({ params }: Props) {
  const { account, id } = await params;
  const nextPath = `/a/${account}/campaigns/${id}`;
  const [c, { me }] = await Promise.all([
    engine<Schemas["Campaign"]>(`/campaigns/${id}`, { account, nextPath }),
    currentAccount(account, nextPath),
  ]);
  const editable = c.status === "draft" || c.status === "scheduled";
  return (
    <>
      <BackLink href={`/a/${account}/campaigns`}>Campaigns</BackLink>
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-semibold">{c.subject || "Untitled draft"}</h1>
        <CampaignStatusBadge status={c.status} />
      </div>
      {editable ? (
        <CampaignEditor account={account} campaign={c} me={me.user.email} />
      ) : (
        <p className="text-sm text-muted-foreground">This issue is {c.status}; it can no longer be edited.</p>
      )}
    </>
  );
}
