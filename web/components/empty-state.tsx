import Link from "next/link";
import { buttonVariants } from "@/components/ui/button";

/** Shown in place of an empty table: one line on why the list matters, and a link to what fills it (left out when that form is already on screen). */
export function EmptyState({ title, hint, action, href }: { title: string; hint: string; action?: string; href?: string }) {
  return (
    <div className="sketch sketch-lg flex flex-col items-start gap-3 bg-card p-6">
      <div className="flex flex-col gap-1">
        <p className="font-heading text-lg font-semibold">{title}</p>
        <p className="max-w-prose text-sm text-muted-foreground">{hint}</p>
      </div>
      {action && href && <Link href={href} className={buttonVariants({ variant: "outline", size: "sm" })}>{action}</Link>}
    </div>
  );
}
