"use client";
import { ConfirmAction } from "@/components/confirm-action";
import type { Schemas } from "@/lib/engine";
import { changeStatus, setArchiveHidden } from "./actions";

export function Controls({ account, campaign }: { account: string; campaign: Schemas["Campaign"] }) {
  const s = campaign.status;
  return (
    <div className="flex flex-wrap gap-2">
      {s === "sending" && (
        <ConfirmAction trigger="Pause" title="Pause this issue?" description="No new waves start. Emails not yet handed to the sender are taken back; Resume sends them later." confirm="Pause" action={async () => { await changeStatus(account, campaign.id, "pause"); }} />
      )}
      {s === "paused" && (
        <ConfirmAction trigger="Resume" title="Resume this issue?" description="The checks run again, then sending continues where it stopped. Nobody gets it twice." confirm="Resume" action={async () => { await changeStatus(account, campaign.id, "resume"); }} />
      )}
      {(s === "sending" || s === "paused" || s === "scheduled") && (
        <ConfirmAction trigger="Cancel" title="Cancel this issue?" description="Readers who have not got it yet never will. This cannot be undone." confirm="Cancel issue" dismiss="Keep it" action={async () => { await changeStatus(account, campaign.id, "cancel"); }} />
      )}
      {s === "sent" && (
        <form action={setArchiveHidden.bind(null, account, campaign.id, !campaign.archive_hidden)}>
          <button type="submit" className="text-sm underline underline-offset-2">{campaign.archive_hidden ? "Show in the public archive" : "Hide from the public archive"}</button>
        </form>
      )}
    </div>
  );
}
