import { Suspense } from "react";
import { ConfirmForm } from "./confirm-form";

export const metadata = { title: "Confirm your subscription" };

type Props = { params: Promise<{ token: string }> };

export default function Confirm({ params }: Props) {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-4 px-4">
      {/* A button, not an automatic confirm: mail scanners open links, people press buttons. The heading follows the state, so it lives in the form. */}
      <Suspense>
        <Form params={params} />
      </Suspense>
    </main>
  );
}

async function Form({ params }: Props) {
  const { token } = await params;
  return <ConfirmForm token={token} />;
}
