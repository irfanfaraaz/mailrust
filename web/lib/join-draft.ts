/** The join form's typed values, kept for one retry after the engine rejects the email. */
export const DRAFT_COOKIE = "join_draft";

export function readDraft(raw: string | undefined): { email: string; first_name: string } {
  try {
    const d = JSON.parse(raw ?? "{}");
    return { email: typeof d.email === "string" ? d.email : "", first_name: typeof d.first_name === "string" ? d.first_name : "" };
  } catch {
    return { email: "", first_name: "" };
  }
}
