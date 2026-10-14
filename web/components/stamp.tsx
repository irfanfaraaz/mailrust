"use client";
import { useEffect, useRef } from "react";

export type Tone = "good" | "wait" | "bad" | "info" | "quiet";

// One colour language for every status: green done, amber waiting or delayed, red needs attention, navy in progress.
const tones: Record<Tone, string> = {
  good: "text-good",
  wait: "text-wait",
  bad: "text-destructive",
  info: "text-primary",
  quiet: "text-muted-foreground",
};

/** A rubber stamp: mono uppercase, double rule, a slight tilt. */
const stamp =
  "inline-flex w-fit shrink-0 -rotate-2 items-center rounded-[2px] border-[1.5px] border-current px-1.5 py-px font-mono text-[0.6875rem] font-bold tracking-[0.08em] whitespace-nowrap uppercase outline-1 outline-offset-[1.5px] outline-current/40";

/** A rubber-stamp label in one of the status tones. It lands again when its text changes (a status moving on). */
export function Stamp({ tone, title, children }: { tone: Tone; title?: string; children: React.ReactNode }) {
  const ref = useRef<HTMLSpanElement>(null);
  const text = typeof children === "string" ? children : null;
  const last = useRef(text);
  useEffect(() => {
    const el = ref.current;
    if (!el || last.current === text) return;
    last.current = text;
    el.classList.remove("stamp-thunk");
    void el.offsetWidth; // restart the animation
    el.classList.add("stamp-thunk");
  }, [text]);
  return <span ref={ref} className={`${stamp} ${tones[tone]}`} title={title}>{children}</span>;
}
