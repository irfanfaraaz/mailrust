import { Suspense } from "react";
import { ConfirmForm } from "./confirm-form";

type Search = Promise<{ token?: string; next?: string }>;

export default function Confirm({ searchParams }: { searchParams: Search }) {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-4 px-4">
      <h1 className="text-2xl font-semibold">Log in to Mailrust</h1>
      {/* searchParams are request data: read them inside Suspense (cacheComponents) */}
      <Suspense>
        <Form searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function Form({ searchParams }: { searchParams: Search }) {
  const { token = "", next = "" } = await searchParams;
  return <ConfirmForm token={token} next={next} />;
}
