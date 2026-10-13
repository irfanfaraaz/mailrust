import Link from "next/link";

export function BackLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <Link href={href} className="self-start text-sm text-muted-foreground underline-offset-2 hover:text-foreground hover:underline">
      ← {children}
    </Link>
  );
}
