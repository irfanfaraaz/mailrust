import { cookies } from "next/headers";
import { Suspense } from "react";
import { Card } from "@/components/ui/card";
import { SESSION_COOKIE } from "@/lib/session-cookie";
import { InviteForm } from "./invite-form";

export const metadata = { title: "Accept invite" };

type Search = Promise<{ token?: string }>;

export default function Invite({ searchParams }: { searchParams: Search }) {
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-sm flex-col justify-center gap-5 px-4 py-10">
      <p className="text-center font-heading text-xl font-semibold">Mailrust</p>
      <Card className="gap-4 px-6 py-7">
        <h1 className="text-2xl font-semibold">Join on Mailrust</h1>
        {/* searchParams and cookies are request data: read them inside Suspense (cacheComponents) */}
        <Suspense>
          <Form searchParams={searchParams} />
        </Suspense>
      </Card>
    </main>
  );
}

async function Form({ searchParams }: { searchParams: Search }) {
  const { token = "" } = await searchParams;
  const loggedIn = (await cookies()).has(SESSION_COOKIE);
  return (
    <>
      {loggedIn && (
        <p className="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">You are logged in. Accepting logs you in as the invited person instead.</p>
      )}
      <InviteForm token={token} />
    </>
  );
}
