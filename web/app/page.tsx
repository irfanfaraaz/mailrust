import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { engine, type Schemas } from "@/lib/engine";

// shortcut: inlines Task 10's getMe("/") (web/lib/account.ts); switch to it when Task 10 lands.
function getMe(nextPath: string) {
  return engine<Schemas["MeResponse"]>("/me", { nextPath });
}

export default function Home() {
  // Cookies and the engine call are request data: read them inside Suspense (cacheComponents).
  return (
    <Suspense>
      <Landing />
    </Suspense>
  );
}

async function Landing() {
  const me = await getMe("/");
  const last = (await cookies()).get("mr_last_account")?.value;
  const target = me.accounts.find((a) => a.id === last) ?? me.accounts[0];
  if (!target) {
    return (
      <main className="mx-auto max-w-md px-4 py-24">
        <h1 className="text-2xl font-semibold">No account yet</h1>
        <p className="mt-2">You are logged in as {me.user.email}, but no account has added you. Ask an owner for an invite.</p>
      </main>
    );
  }
  redirect(`/a/${target.id}/overview`);
}
