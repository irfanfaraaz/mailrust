import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { Landing as FrontPage } from "@/components/landing/landing";
import { Card } from "@/components/ui/card";
import { getMe } from "@/lib/account";
import { SESSION_COOKIE } from "@/lib/session-cookie";

export default function Home() {
  // Cookies and the engine call are request data: read them inside Suspense (cacheComponents).
  return (
    <Suspense>
      <Landing />
    </Suspense>
  );
}

async function Landing() {
  const jar = await cookies();
  // Only our own public deploy shows the front page; self-hosted copies keep "/" as the way in.
  const landing = process.env.MAILRUST_LANDING;
  if (landing === "only" || (landing === "true" && !jar.has(SESSION_COOKIE))) return <FrontPage />;
  const me = await getMe("/");
  const last = jar.get("mr_last_account")?.value;
  const target = me.accounts.find((a) => a.id === last) ?? me.accounts[0];
  if (!target) {
    return (
      <main className="mx-auto flex min-h-dvh w-full max-w-md flex-col justify-center px-4 py-10">
        <Card className="gap-3 px-6 py-7">
          <h1 className="text-2xl font-semibold">No account yet</h1>
          <p>You are logged in as {me.user.email}, but no account has added you. Ask an owner for an invite.</p>
        </Card>
      </main>
    );
  }
  redirect(`/a/${target.id}/overview`);
}
