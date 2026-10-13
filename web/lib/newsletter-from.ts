/**
 * The newsletter_from to PATCH, or undefined to keep the saved one: unchanged, or the form had no
 * from inputs (no verified domain left). Never wipes or swaps the address behind the user's back.
 */
export function changedFrom(form: FormData): string | undefined {
  if (!form.has("from_local")) return undefined;
  const local = String(form.get("from_local") ?? "").trim();
  const domain = String(form.get("from_domain") ?? "");
  const next = local && domain ? `${local}@${domain}` : "";
  return next === String(form.get("from_current") ?? "") ? undefined : next;
}
