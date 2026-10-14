import Link from "next/link";

export function PausedBanner({ accountId, reason }: { accountId: string; reason: string }) {
  return (
    <div role="alert" className="border-b-2 border-destructive bg-destructive/10 px-4 py-3 text-sm text-destructive md:px-8">
      <p>
        <strong>Sending is paused for this account.</strong> Reason: {reason}.
      </p>
      <p className="mt-1">
        Mailrust pauses an account when its bounce or complaint rate goes over the limit, or when an operator pauses it. New emails are
        refused until an operator resumes the account.{" "}
        <Link href={`/a/${accountId}/overview`} className="font-medium underline underline-offset-2">
          Check your rates on Overview
        </Link>
      </p>
    </div>
  );
}
