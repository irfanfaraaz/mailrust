/** A contact's fields as text: one name=value per line. Blank lines are skipped; the first "=" splits. */
export function parseFields(text: string): { value: Record<string, string> } | { error: string } {
  const value: Record<string, string> = {};
  for (const [i, raw] of text.split(/\r?\n/).entries()) {
    const line = raw.trim();
    if (!line) continue;
    const eq = line.indexOf("=");
    if (eq <= 0) return { error: `Line ${i + 1}: write each field as name=value.` };
    value[line.slice(0, eq).trim()] = line.slice(eq + 1).trim();
  }
  return { value };
}

export const formatFields = (f: Record<string, string>) => Object.entries(f).map(([k, v]) => `${k}=${v}`).join("\n");
