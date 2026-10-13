export function PausedBanner({ reason }: { accountId: string; reason: string }) {
  return (
    <div role="alert" className="border-b border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive md:px-8">
      <strong>Sending is paused for this account.</strong> {reason}. New emails are refused until an operator resumes the account.
    </div>
  );
}
