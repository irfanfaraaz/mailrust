import Link from "next/link";

// Lives above [account] so it also catches notFound() from the [account] layout (not-found.js sits inside its own segment's layout).
export default function NotFound() {
  return (
    <main className="mx-auto max-w-md px-4 py-24">
      <h1 className="text-2xl font-semibold">Not found</h1>
      <p className="mt-2">This page or account does not exist, or you are not a member.</p>
      <Link href="/" className="mt-4 inline-block underline">Go to my account</Link>
    </main>
  );
}
