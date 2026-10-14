"use client";
import { usePathname } from "next/navigation";
import { useLayoutEffect, useRef } from "react";

// The sidebar group each section belongs to, so the underline wears that group's colour.
const groupTab: Record<string, string> = {
  campaigns: "var(--tab-newsletter)",
  contacts: "var(--tab-newsletter)",
  tags: "var(--tab-newsletter)",
  "signup-form": "var(--tab-newsletter)",
  emails: "var(--tab-api)",
  "api-keys": "var(--tab-api)",
  webhooks: "var(--tab-api)",
  domains: "var(--tab-settings)",
  suppressions: "var(--tab-settings)",
  team: "var(--tab-settings)",
};

// Pages already seen in this tab: the line is drawn in only the first time.
const seen = new Set<string>();

/** A pen underline under the page title, in the colour of the page's sidebar group. */
export function TitleScribble() {
  const pathname = usePathname();
  const ref = useRef<SVGSVGElement>(null);
  const mine = useRef<string | null>(null); // the page this underline already decided for (effects can run twice in dev)
  useLayoutEffect(() => {
    if (mine.current === pathname) return;
    mine.current = pathname;
    if (seen.has(pathname)) ref.current?.classList.remove("scribble-draw");
    seen.add(pathname);
  }, [pathname]);
  const section = pathname.split("/")[3] ?? "";
  return (
    <svg
      ref={ref}
      aria-hidden
      viewBox="0 0 200 12"
      preserveAspectRatio="none"
      className="scribble scribble-draw pointer-events-none absolute -bottom-2 -left-1 h-3 w-[calc(100%+0.5rem)] overflow-visible"
      style={{ "--scribble": groupTab[section] ?? "var(--primary)" } as React.CSSProperties}
    >
      <path d="M2 8 C 40 2, 80 11, 120 6 S 180 3, 198 7" />
    </svg>
  );
}
