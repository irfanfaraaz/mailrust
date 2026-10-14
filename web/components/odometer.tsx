/** A figure whose digits roll like a mechanical counter when the value changes (a refresh). The first render just shows it. */
export function Odometer({ value }: { value: string }) {
  const chars = [...value];
  return (
    <span className="odo">
      <span className="sr-only">{value}</span>
      {/* Keyed from the right, so 999 → 1,000 keeps the ones, tens and hundreds columns in place. */}
      {chars.map((ch, i) => {
        const key = chars.length - i;
        return /\d/.test(ch)
          ? <span key={key} aria-hidden className="odo-col" style={{ translate: `0 -${Number(ch) * 1.2}em` }} />
          : <span key={key} aria-hidden>{ch}</span>;
      })}
    </span>
  );
}
