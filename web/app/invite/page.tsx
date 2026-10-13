import { Suspense } from "react";
import { InviteForm } from "./invite-form";

type Search = Promise<{ token?: string }>;

export default function Invite({ searchParams }: { searchParams: Search }) {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-4 px-4">
      <h1 className="text-2xl font-semibold">Join on Mailrust</h1>
      {/* searchParams are request data: read them inside Suspense (cacheComponents) */}
      <Suspense>
        <Form searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function Form({ searchParams }: { searchParams: Search }) {
  const { token = "" } = await searchParams;
  return <InviteForm token={token} />;
}
