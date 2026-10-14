"use client";
import { BanIcon, FileTextIcon, GlobeIcon, HouseIcon, KeyRoundIcon, MailIcon, SendIcon, TagIcon, UserIcon, UsersIcon, WebhookIcon, type LucideIcon } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef } from "react";
import { cn } from "cn";

type Item = readonly [name: string, slug: string, icon: LucideIcon];

// Newsletter first, the sending API below it, shared settings last. Each group has its own colour tab.
const groups: readonly { label: string | null; tab: string; items: readonly Item[] }[] = [
  { label: null, tab: "var(--primary)", items: [["Overview", "overview", HouseIcon]] },
  {
    label: "Newsletter",
    tab: "var(--tab-newsletter)",
    items: [["Campaigns", "campaigns", SendIcon], ["Contacts", "contacts", UsersIcon], ["Tags", "tags", TagIcon], ["Signup form", "signup-form", FileTextIcon]],
  },
  { label: "Sending API", tab: "var(--tab-api)", items: [["Emails", "emails", MailIcon], ["API keys", "api-keys", KeyRoundIcon], ["Webhooks", "webhooks", WebhookIcon]] },
  { label: "Settings", tab: "var(--tab-settings)", items: [["Domains", "domains", GlobeIcon], ["Suppressions", "suppressions", BanIcon], ["Team", "team", UserIcon]] },
];

export function Sidebar({ accountId }: { accountId: string }) {
  const pathname = usePathname();
  const nav = useRef<HTMLElement>(null);
  // Phones: keep the current page in view in the scrolling row.
  useEffect(() => {
    nav.current?.querySelector("[aria-current=page]")?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [pathname]);
  return (
    // Phones: one scrolling row of links above the content; md+: grouped column beside it.
    <nav ref={nav} aria-label="Main" className="flex gap-1 overflow-x-auto px-3 pb-2 [scrollbar-width:none] max-md:[mask-image:linear-gradient(to_right,black_85%,transparent)] md:flex-col md:gap-5 md:overflow-visible md:px-3 md:py-2">
      {groups.map((g) => (
        <div key={g.label ?? "top"} style={{ "--tab": g.tab } as React.CSSProperties} className="flex shrink-0 gap-1 md:flex-col md:gap-0.5">
          {g.label && (
            <p className="label-caps mb-1 hidden items-center gap-2 px-2 text-[color-mix(in_oklab,var(--tab)_55%,var(--foreground))] md:flex">
              <span aria-hidden className="size-2.5 rounded-[2px] bg-(--tab)" />
              {g.label}
            </p>
          )}
          {g.items.map(([name, slug, Icon]) => {
            const href = `/a/${accountId}/${slug}`;
            // Detail pages (emails/<id>) keep their section highlighted.
            const active = pathname === href || pathname.startsWith(`${href}/`);
            return (
              <Link
                key={slug}
                href={href}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "flex min-h-11 shrink-0 items-center gap-2.5 whitespace-nowrap rounded-md px-3 text-[0.9375rem] transition-colors md:min-h-0 md:px-2 md:py-2",
                  active
                    ? "bg-sidebar-accent font-semibold text-foreground md:shadow-[inset_3px_0_0_var(--tab)]"
                    : "text-muted-foreground hover:bg-sidebar-accent/70 hover:text-foreground",
                )}
              >
                <Icon aria-hidden className="size-4 shrink-0" />
                {name}
              </Link>
            );
          })}
        </div>
      ))}
    </nav>
  );
}
