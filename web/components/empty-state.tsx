import { buttonVariants } from "@/components/ui/button";

/** Shown in place of an empty table: one line on why the list matters, and a link to the form that fills it. */
export function EmptyState({ title, hint, action, href }: { title: string; hint: string; action: string; href: string }) {
  return (
    <div className="flex flex-col items-start gap-3 rounded-lg border border-dashed p-6">
      <div className="flex flex-col gap-1">
        <p className="font-medium">{title}</p>
        <p className="max-w-prose text-sm text-muted-foreground">{hint}</p>
      </div>
      <a href={href} className={buttonVariants({ variant: "outline", size: "sm" })}>{action}</a>
    </div>
  );
}
