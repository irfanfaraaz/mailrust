"use client";
import { ActionForm } from "@/components/action-form";
import { ConfirmAction } from "@/components/confirm-action";
import type { Schemas } from "@/lib/engine";
import { changeStatus, setArchiveHidden } from "./actions";

export function Controls({ account, campaign }: { account: string; campaign: Schemas["Campaign"] }) {
  const s = campaign.status;
  return (
    <div className="flex flex-wrap gap-2">
      {s === "sending" && (
        <ConfirmAction trigger="Pause" title="Pause this issue?" description="No new waves start. Emails not yet handed to the sender are taken back; Resume sends them later." confirm="Pause" variant="default" done="Issue paused" action={() => changeStatus(account, campaign.id, "pause")} />
      )}
      {s === "paused" && (
        <ConfirmAction trigger="Resume" title="Resume this issue?" description="The checks run again, then sending continues where it stopped. Nobody gets it twice." confirm="Resume" variant="default" done="Sending resumed" action={() => changeStatus(account, campaign.id, "resume")} />
      )}
      {(s === "sending" || s === "paused" || s === "scheduled") && (
        <ConfirmAction trigger="Cancel" title="Cancel this issue?" description="Readers who have not got it yet never will. This cannot be undone." confirm="Cancel issue" dismiss="Keep it" done="Issue cancelled" action={() => changeStatus(account, campaign.id, "cancel")} />
      )}
      {s === "sent" && (
        <ActionForm action={setArchiveHidden.bind(null, account, campaign.id, !campaign.archive_hidden)} done={campaign.archive_hidden ? "Shown in the public archive" : "Hidden from the public archive"}>
          <button type="submit" className="text-sm text-primary underline underline-offset-2">{campaign.archive_hidden ? "Show in the public archive" : "Hide from the public archive"}</button>
        </ActionForm>
      )}
    </div>
  );
}
