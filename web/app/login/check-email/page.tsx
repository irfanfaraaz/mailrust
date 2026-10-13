export default function CheckEmail() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-3 px-4">
      <h1 className="text-2xl font-semibold">Check your email</h1>
      <p>If that address has a Mailrust login, a link is on its way. It works once and expires in 15 minutes.</p>
      <a className="underline" href="/login">Use another address</a>
    </main>
  );
}
