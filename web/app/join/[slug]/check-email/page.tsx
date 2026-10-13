export const metadata = { title: "Check your email" };

export default function CheckEmail() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-3 px-4">
      <h1 className="text-2xl font-semibold">Check your email</h1>
      <p>We sent you a link to confirm your subscription. It works for 7 days.</p>
      <p className="text-sm text-muted-foreground">Nothing there? Check your spam folder.</p>
    </main>
  );
}
