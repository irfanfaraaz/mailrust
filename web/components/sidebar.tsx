"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "cn";

const groups = [
  { label: null, items: [["Overview", "overview"]] },
  { label: "Sending API", items: [["Emails", "emails"], ["API keys", "api-keys"], ["Webhooks", "webhooks"]] },
  { label: "Newsletter", items: [["Campaigns", "campaigns"], ["Contacts", "contacts"], ["Tags", "tags"], ["Signup form", "signup-form"]] },
  { label: "Shared", items: [["Domains", "domains"], ["Suppressions", "suppressions"], ["Team", "team"]] },
] as const;

export function Sidebar({ accountId }: { accountId: string }) {
  const pathname = usePathname();
  return (
    // Phones: one scrolling row of links above the content; md+: grouped column beside it.
    <nav aria-label="Main" className="flex gap-1 overflow-x-auto px-3 pb-2 md:flex-col md:gap-4 md:overflow-visible md:p-3">
      {groups.map((g) => (
        <div key={g.label ?? "top"} className="flex shrink-0 gap-1 md:flex-col">
          {g.label && <p className="hidden px-2 text-xs font-medium uppercase tracking-wide text-muted-foreground md:block">{g.label}</p>}
          {g.items.map(([name, slug]) => {
            const href = `/a/${accountId}/${slug}`;
            // Detail pages (emails/<id>) keep their section highlighted.
            const active = pathname === href || pathname.startsWith(`${href}/`);
            return (
              <Link
                key={slug}
                href={href}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "flex min-h-11 shrink-0 items-center whitespace-nowrap rounded-md px-3 transition-colors md:min-h-0 md:px-2 md:py-1.5",
                  active ? "bg-muted font-medium text-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                {name}
              </Link>
            );
          })}
        </div>
      ))}
    </nav>
  );
}
