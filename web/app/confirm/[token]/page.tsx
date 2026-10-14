import { Suspense } from "react";
import { Card } from "@/components/ui/card";
import { engine, type Schemas } from "@/lib/engine";
import { ConfirmForm } from "./confirm-form";

export const metadata = { title: "Confirm your subscription" };

type Props = { params: Promise<{ token: string }> };

export default function Confirm({ params }: Props) {
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-sm flex-col justify-center px-4 py-10">
      {/* A button, not an automatic confirm: mail scanners open links, people press buttons. The heading follows the state, so it lives in the form. */}
      <Card className="gap-4 px-6 py-7">
        <Suspense>
          <Form params={params} />
        </Suspense>
      </Card>
    </main>
  );
}

async function Form({ params }: Props) {
  const { token } = await params;
  // The name is a nicety: if the lookup fails (429, outage), still show the Confirm button.
  const info = await engine<Schemas["ConfirmInfo"]>(`/public/confirm/${encodeURIComponent(token)}`, { auth: false }).catch(() => null);
  return <ConfirmForm token={token} name={info?.newsletter_name ?? null} slug={info?.signup_slug ?? null} />;
}
