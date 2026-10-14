"use client";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

/** Refreshes the server-rendered page every 10 s while `active` (a sending issue, the live email log). */
export function AutoRefresh({ active }: { active: boolean }) {
  const router = useRouter();
  useEffect(() => {
    if (!active) return;
    const t = setInterval(() => router.refresh(), 10_000);
    return () => clearInterval(t);
  }, [active, router]);
  return null;
}
