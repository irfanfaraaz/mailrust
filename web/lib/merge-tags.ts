export const BUILT_IN_KEYS = ["first_name", "last_name", "email"] as const;
export const MERGE_LABELS: Record<string, string> = { first_name: "First name", last_name: "Last name", email: "Email" };
export const MAX_FALLBACK = 100;
const FIELD_KEY = /^[a-z0-9_]{1,40}$/;

/** Why this merge tag cannot be inserted, or null. Same rules as the engine (src/render/blocks.rs). */
export function mergeTagProblem(key: string, fallback: string): string | null {
  if (!FIELD_KEY.test(key.trim())) return "Use a field name of a-z, 0-9 and _ (at most 40 characters).";
  const f = fallback.trim();
  if (key.trim() !== "email" && !f) return "Add a fallback: readers without this value see it instead.";
  if ([...f].length > MAX_FALLBACK) return `Keep the fallback under ${MAX_FALLBACK} characters.`;
  return null;
}

/** What the editor chip shows: {{first_name | there}}. */
export function mergeTagLabel(key: string, fallback: string): string {
  const f = fallback.trim();
  return f ? `{{${key} | ${f}}}` : `{{${key}}}`;
}
