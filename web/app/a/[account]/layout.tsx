import { PlusIcon } from "lucide-react";
import { Suspense } from "react";
import { AccountMenu } from "@/components/account-menu";
import { HealthStrip } from "@/components/health-strip";
import { Loading } from "@/components/loading";
import { PausedBanner } from "@/components/paused-banner";
import { RememberAccount } from "@/components/remember-account";
import { Sidebar } from "@/components/sidebar";
import { Button } from "@/components/ui/button";
import { currentAccount } from "@/lib/account";
import { createCampaign } from "./campaigns/actions";

type Props = { children: React.ReactNode; params: Promise<{ account: string }> };

const grid = "grid min-h-dvh md:grid-cols-[248px_1fr]";
// md+: the sidebar stays put while the page scrolls, so sending health is always in view.
const aside = "flex flex-col border-b bg-sidebar md:sticky md:top-0 md:h-dvh md:overflow-y-auto md:border-b-0 md:border-r";
const main = "px-4 py-6 md:px-10 md:py-9";

export default function AccountLayout(props: Props) {
  // params and the engine call are request data: read them inside Suspense (cacheComponents).
  return (
    <Suspense fallback={<ShellFallback />}>
      <Shell {...props} />
    </Suspense>
  );
}

/** Same frame as the shell, so the page does not jump when the account loads. */
function ShellFallback() {
  return (
    <div className={grid}>
      <aside className={aside}>
        <Loading rows={3} className="p-3" />
      </aside>
      <main className={main}>
        <Loading />
      </main>
    </div>
  );
}

async function Shell({ children, params }: Props) {
  const { account: id } = await params;
  const { me, account } = await currentAccount(id, `/a/${id}/overview`);
  return (
    <div className={grid}>
      <aside className={aside}>
        <AccountMenu me={me} current={account} />
        <form action={createCampaign.bind(null, id)} className="hidden px-3 pb-3 md:block">
          <Button type="submit" size="lg" className="w-full">
            <PlusIcon aria-hidden />
            New issue
          </Button>
        </form>
        <Sidebar accountId={id} />
        <div className="mt-auto hidden md:block">
          <Suspense fallback={<Loading rows={1} className="m-3" />}>
            <HealthStrip accountId={id} />
          </Suspense>
        </div>
      </aside>
      <div className="min-w-0">
        {account.status === "paused" && <PausedBanner accountId={id} reason={account.paused_reason ?? "no reason given"} />}
        <main className={main}>{children}</main>
      </div>
      <RememberAccount id={id} />
    </div>
  );
}
