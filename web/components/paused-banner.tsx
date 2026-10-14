import Link from "next/link";

/** Calm and specific: say why, circle the number that caused it in red pen, then list what to do. */
export function PausedBanner({ accountId, reason }: { accountId: string; reason: string }) {
  // Engine reasons read like "bounce rate 2.40% is over 2% in the last 7 days (31 of 1290 recipients)": the first % is the cause.
  const m = /\d+(?:\.\d+)?%/.exec(reason);
  const rate = m !== null;
  return (
    <div role="alert" className="border-b-2 border-destructive bg-destructive/10 px-4 py-3 text-sm md:px-8">
      <p>
        <strong className="text-destructive">Sending is paused to protect your domain.</strong> Reason:{" "}
        {m ? (
          <>
            {reason.slice(0, m.index)}
            <span className="pen-circle relative inline-block px-1">
              <span className="font-mono font-semibold">{m[0]}</span>
              <svg viewBox="0 0 60 30" preserveAspectRatio="none" aria-hidden className="pointer-events-none absolute -inset-x-2.5 -inset-y-2 h-[calc(100%+1rem)] w-[calc(100%+1.25rem)] overflow-visible">
                <path pathLength={1} d="M8 6 C 30 -2, 58 4, 56 16 C 54 28, 14 30, 4 20 C -2 12, 12 4, 26 3" />
              </svg>
            </span>
            {reason.slice(m.index + m[0].length)}
          </>
        ) : reason}
        .
      </p>
      {rate ? (
        <ol className="mt-2 flex list-decimal flex-col gap-0.5 pl-5">
          <li>See which addresses bounced or complained on <Link href={`/a/${accountId}/suppressions`} className="font-medium underline underline-offset-2">Suppressions</Link>. Hard bounces and complaints are blocked already.</li>
          <li>Check your newest contact import for typos and old addresses.</li>
          <li>Ask an operator to resume the account. New emails are refused until then.</li>
        </ol>
      ) : (
        <p className="mt-1">
          New emails are refused until an operator resumes the account.{" "}
          <Link href={`/a/${accountId}/overview`} className="font-medium underline underline-offset-2">Check your rates on Overview</Link>
        </p>
      )}
    </div>
  );
}
