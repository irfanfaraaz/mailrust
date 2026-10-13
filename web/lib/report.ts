const n = (x: number) => x.toLocaleString("en-US");

/** 0.25 → "25%", 0.0312 → "3.1%". */
export function pct(rate: number): string {
  const p = rate * 100;
  return `${p > 0 && p < 10 && !Number.isInteger(p) ? p.toFixed(1) : Math.round(p)}%`;
}

export function wavePlanText(sizes: number[]): string {
  if (sizes.length === 0) return "No readers match this audience.";
  if (sizes.length === 1) return `One wave: all ${n(sizes[0])} readers at once.`;
  return `${sizes.length} waves, 10 minutes apart: ${sizes.map(n).join(", ")}. Most engaged readers go first.`;
}

/** "312 of 1,200 have no first name; they will see “there”." — only when the issue greets by first name. */
export function namesMissingText(missing: number, total: number, fallback: string | null): string | null {
  if (!fallback || missing === 0) return null;
  return `${n(missing)} of ${n(total)} have no first name; they will see “${fallback}”.`;
}
