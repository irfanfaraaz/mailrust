"use client";
import { BanIcon, FileTextIcon, GlobeIcon, HouseIcon, KeyRoundIcon, MailIcon, SendIcon, TagIcon, UserIcon, UsersIcon, WebhookIcon, type LucideIcon } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";
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
  // md+: one highlight that slides to the current page and takes its group's colour.
  const [mark, setMark] = useState<{ top: number; height: number; tab: string } | null>(null);
  useEffect(() => {
    const n = nav.current;
    const active = n?.querySelector<HTMLElement>("[aria-current=page]");
    // Phones: keep the current page in view in the scrolling row.
    active?.scrollIntoView({ block: "nearest", inline: "nearest" });
    if (!n || !active) return setMark(null);
    const place = () => setMark({ top: active.offsetTop, height: active.offsetHeight, tab: active.dataset.tab ?? "var(--primary)" });
    place();
    const ro = new ResizeObserver(place);
    ro.observe(n);
    return () => ro.disconnect();
  }, [pathname]);
  return (
    // Phones: one scrolling row of links above the content; md+: grouped column beside it.
    <nav ref={nav} aria-label="Main" className="relative flex gap-1 overflow-x-auto px-3 pb-2 [scrollbar-width:none] max-md:[mask-image:linear-gradient(to_right,black_85%,transparent)] md:flex-col md:gap-5 md:overflow-visible md:px-3 md:py-2">
      {mark && (
        <span
          aria-hidden
          className="pointer-events-none absolute inset-x-3 top-0 hidden rounded-md bg-sidebar-accent transition-[translate,height,box-shadow] duration-300 ease-[cubic-bezier(0.22,1,0.36,1)] motion-reduce:transition-none md:block"
          style={{ translate: `0 ${mark.top}px`, height: mark.height, boxShadow: `inset 3px 0 0 ${mark.tab}` }}
        />
      )}
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
                data-tab={g.tab}
                className={cn(
                  "relative flex min-h-11 shrink-0 items-center gap-2.5 whitespace-nowrap rounded-md px-3 text-[0.9375rem] transition-colors md:min-h-0 md:px-2 md:py-2",
                  active
                    ? cn("font-semibold text-foreground max-md:bg-sidebar-accent", !mark && "md:bg-sidebar-accent md:shadow-[inset_3px_0_0_var(--tab)]")
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
