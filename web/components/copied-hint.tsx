/** A small tilted "Copied" that floats above a copy button. Bump `n` on each copy to play it again. Put it in a `relative` box. */
export function CopiedHint({ n }: { n: number }) {
  return n > 0 ? <span key={n} aria-hidden className="copied-hint">✓ Copied</span> : null;
}
