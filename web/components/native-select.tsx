import { cn } from "cn";

/** A styled native <select>: works in no-JS GET forms and with Playwright's selectOption. Matches the house Input. */
export function NativeSelect({ className, ...props }: React.ComponentProps<"select">) {
  return (
    <select
      className={cn(
        "h-8 rounded-lg border border-input bg-transparent px-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50 max-md:h-11 dark:bg-input/30",
        className,
      )}
      {...props}
    />
  );
}
