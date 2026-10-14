"use client";
import Link from "next/link";
import { useLayoutEffect, useRef } from "react";
import { morphFrom, rememberFrom } from "@/lib/morph";

/** A link whose target page can grow out of it (see MorphTitle). */
export function MorphLink({ morph, ...props }: React.ComponentProps<typeof Link> & { morph: string }) {
  return <Link {...props} onClick={(e) => { rememberFrom(morph, e.currentTarget); props.onClick?.(e); }} />;
}

/** A page title that flies in from the MorphLink with the same key. */
export function MorphTitle({ morph, children }: { morph: string; children: React.ReactNode }) {
  const ref = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => { if (ref.current) morphFrom(morph, ref.current, { uniform: true }); }, [morph]);
  return <span ref={ref} className="inline-block">{children}</span>;
}
