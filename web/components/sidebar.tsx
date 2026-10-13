import Link from "next/link";

const groups = [
  { label: null, items: [["Overview", "overview"]] },
  { label: "Sending API", items: [["Emails", "emails"], ["API keys", "api-keys"], ["Webhooks", "webhooks"]] },
  // Parts 2 and 3 add: { label: "Newsletter", items: [["Audience", "audience"], ["Campaigns", "campaigns"]] }
  { label: "Shared", items: [["Domains", "domains"], ["Suppressions", "suppressions"], ["Team", "team"]] },
] as const;

export function Sidebar({ accountId }: { accountId: string }) {
  return (
    <nav aria-label="Main" className="flex flex-col gap-4 p-3">
      {groups.map((g) => (
        <div key={g.label ?? "top"} className="flex flex-col gap-1">
          {g.label && <p className="px-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{g.label}</p>}
          {g.items.map(([name, slug]) => (
            <Link key={slug} href={`/a/${accountId}/${slug}`} className="rounded px-2 py-1.5 hover:bg-muted">{name}</Link>
          ))}
        </div>
      ))}
    </nav>
  );
}
