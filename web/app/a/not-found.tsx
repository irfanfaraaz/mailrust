import Link from "next/link";
import { Card } from "@/components/ui/card";

// Lives above [account] so it also catches notFound() from the [account] layout (not-found.js sits inside its own segment's layout).
export default function NotFound() {
  return (
    <main className="mx-auto flex min-h-dvh w-full max-w-md flex-col justify-center px-4 py-10">
      <Card className="gap-3 px-6 py-7">
        <h1 className="text-2xl font-semibold">Not found</h1>
        <p>This page or account does not exist, or you are not a member.</p>
        <Link href="/" className="text-primary underline underline-offset-2">Go to my account</Link>
      </Card>
    </main>
  );
}
