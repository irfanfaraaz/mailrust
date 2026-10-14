import Link from "next/link";
import { buttonVariants } from "@/components/ui/button";

/** Shown in place of an empty table: one line on why the list matters, and a link to what fills it (left out when that form is already on screen). */
export function EmptyState({ title, hint, action, href }: { title: string; hint: string; action?: string; href?: string }) {
  return (
    <div className="sketch sketch-lg flex flex-wrap items-center gap-x-6 gap-y-4 bg-card p-6">
      <Doodle />
      <div className="flex min-w-0 flex-[1_1_16rem] flex-col items-start gap-3">
        <div className="flex flex-col gap-1">
          <p className="font-heading text-lg font-semibold">{title}</p>
          <p className="max-w-prose text-sm text-muted-foreground">{hint}</p>
        </div>
        {action && href && <Link href={href} className={buttonVariants({ variant: "outline", size: "sm" })}>{action}</Link>}
      </div>
    </div>
  );
}

/** A pen drawing of an envelope that draws itself in; still when motion is reduced. */
function Doodle() {
  const stroke = (len: number, i: number) => ({ "--len": len, "--i": i }) as React.CSSProperties;
  return (
    <svg aria-hidden className="doodle shrink-0" width="110" height="86" viewBox="0 0 110 86">
      <rect x="8" y="16" width="78" height="56" rx="4" style={stroke(270, 0)} />
      <path d="M8 20 L47 48 L86 20" style={stroke(110, 1)} />
      <path className="doodle-accent" d="M70 8 q14 -6 26 6 q6 8 -2 16" style={stroke(60, 2)} />
      <line x1="20" y1="64" x2="44" y2="64" style={stroke(26, 3)} />
      <path className="doodle-accent" d="M92 34 l6 -4 M94 42 l8 0" style={stroke(20, 4)} />
    </svg>
  );
}
