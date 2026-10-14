import { Card } from "@/components/ui/card";

export const metadata = { title: "Check your email" };

export default function CheckEmail() {
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-sm flex-col justify-center gap-5 px-4 py-10">
      <p className="text-center font-heading text-xl font-semibold">Mailrust</p>
      <Card className="gap-4 px-6 py-7">
        <h1 className="text-2xl font-semibold">Check your email</h1>
        <p>If that address has a Mailrust login, a link is on its way. It works once and expires in 15 minutes.</p>
        <a className="text-sm text-primary underline underline-offset-2" href="/login">Use another address</a>
      </Card>
    </main>
  );
}
