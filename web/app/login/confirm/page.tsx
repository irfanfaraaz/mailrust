import { Suspense } from "react";
import { Card } from "@/components/ui/card";
import { ConfirmForm } from "./confirm-form";

export const metadata = { title: "Log in" };

type Search = Promise<{ token?: string; next?: string }>;

export default function Confirm({ searchParams }: { searchParams: Search }) {
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-sm flex-col justify-center gap-5 px-4 py-10">
      <p className="text-center font-heading text-xl font-semibold">Mailrust</p>
      <Card className="gap-4 px-6 py-7">
        <h1 className="text-2xl font-semibold">Log in to Mailrust</h1>
        {/* searchParams are request data: read them inside Suspense (cacheComponents) */}
        <Suspense>
          <Form searchParams={searchParams} />
        </Suspense>
      </Card>
    </main>
  );
}

async function Form({ searchParams }: { searchParams: Search }) {
  const { token = "", next = "" } = await searchParams;
  return <ConfirmForm token={token} next={next} />;
}
