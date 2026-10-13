import { Suspense } from "react";
import { AccountMenu } from "@/components/account-menu";
import { PausedBanner } from "@/components/paused-banner";
import { RememberAccount } from "@/components/remember-account";
import { Sidebar } from "@/components/sidebar";
import { currentAccount } from "@/lib/account";

type Props = { children: React.ReactNode; params: Promise<{ account: string }> };

export default function AccountLayout(props: Props) {
  // params and the engine call are request data: read them inside Suspense (cacheComponents).
  return (
    <Suspense>
      <Shell {...props} />
    </Suspense>
  );
}

async function Shell({ children, params }: Props) {
  const { account: id } = await params;
  const { me, account } = await currentAccount(id, `/a/${id}/overview`);
  return (
    <div className="grid min-h-dvh md:grid-cols-[240px_1fr]">
      <aside className="border-b md:border-b-0 md:border-r">
        <AccountMenu me={me} current={account} />
        <Sidebar accountId={id} />
      </aside>
      <div className="min-w-0">
        {account.status === "paused" && <PausedBanner accountId={id} reason={account.paused_reason ?? "no reason given"} />}
        <main className="px-4 py-6 md:px-8">{children}</main>
      </div>
      <RememberAccount id={id} />
    </div>
  );
}
