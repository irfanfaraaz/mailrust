"use client";
import { useSearchParams } from "next/navigation";
import { Suspense, useActionState } from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { requestLink, type FormState } from "./actions";

export default function LoginPage() {
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-sm flex-col justify-center gap-5 px-4 py-10">
      <p className="text-center font-heading text-xl font-semibold">Mailrust</p>
      <Card className="gap-4 px-6 py-7">
        <h1 className="text-2xl font-semibold">Log in to Mailrust</h1>
        {/* useSearchParams needs a Suspense boundary under cacheComponents */}
        <Suspense>
          <LoginForm />
        </Suspense>
      </Card>
    </main>
  );
}

function LoginForm() {
  const next = useSearchParams().get("next") ?? "";
  const [state, action, pending] = useActionState<FormState, FormData>(requestLink, {});
  return (
    <form action={action} className="flex flex-col gap-3">
      <input type="hidden" name="next" value={next} />
      <Label htmlFor="email">Email</Label>
      <Input id="email" name="email" type="email" autoComplete="email" required />
      {state.error && <p role="alert" className="text-sm text-destructive">{state.error}</p>}
      <Button type="submit" disabled={pending}>Send login link</Button>
    </form>
  );
}
