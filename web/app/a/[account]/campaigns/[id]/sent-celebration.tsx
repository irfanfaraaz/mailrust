"use client";
import { SendIcon } from "lucide-react";
import { useEffect, useState } from "react";

const KEY = "mr_sent";
const EVENT = "mr:sent";

/** Call after "Send now" works. The page may swap the editor for the report, so the moment is handed over through sessionStorage. */
export function celebrateSend(campaignId: string, readers: number) {
  try {
    sessionStorage.setItem(KEY, JSON.stringify({ id: campaignId, readers }));
  } catch {
    /* storage blocked: the event below still plays it while this page stays mounted */
  }
  window.dispatchEvent(new CustomEvent(EVENT, { detail: { id: campaignId, readers } }));
}

const flecks = Array.from({ length: 9 }, (_, k) => {
  const a = (k / 9) * Math.PI * 2;
  const d = 48 + (k % 3) * 12;
  return { "--dx": `${Math.cos(a) * d}px`, "--dy": `${Math.sin(a) * d}px` } as React.CSSProperties;
});

/** The big "Sent" stamp, ink flecks and a paper plane, once, right after an issue starts sending. */
export function SentCelebration({ campaignId }: { campaignId: string }) {
  const [readers, setReaders] = useState<number | null>(null);
  useEffect(() => {
    const play = (d: { id: string; readers: number } | null) => {
      if (!d || d.id !== campaignId || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
      try {
        sessionStorage.removeItem(KEY);
      } catch {}
      setReaders(d.readers);
    };
    try {
      play(JSON.parse(sessionStorage.getItem(KEY) ?? "null"));
    } catch {}
    const on = (e: Event) => play((e as CustomEvent).detail);
    window.addEventListener(EVENT, on);
    return () => window.removeEventListener(EVENT, on);
  }, [campaignId]);
  useEffect(() => {
    if (readers === null) return;
    const t = setTimeout(() => setReaders(null), 2100);
    return () => clearTimeout(t);
  }, [readers]);
  if (readers === null) return null;
  return (
    <div aria-hidden className="fade-away pointer-events-none fixed inset-0 z-50 grid place-items-center">
      <div className="relative flex flex-col items-center gap-4 rounded-2xl bg-background/85 px-10 py-8 backdrop-blur-sm">
        <SendIcon className="plane-fly size-9 text-primary" strokeWidth={1.8} />
        <span className="relative">
          <span className="sent-slam inline-flex -rotate-3 rounded-[3px] border-[3px] border-current px-3.5 py-1.5 font-mono text-xl font-bold tracking-[0.08em] text-primary uppercase outline-[1.5px] outline-offset-[3px] outline-current/40">
            Sent · Wave 1
          </span>
          {flecks.map((style, k) => (
            <span key={k} style={style} className="ink-fleck absolute top-1/2 left-1/2 size-1.5 rounded-full bg-primary opacity-0" />
          ))}
        </span>
        <span className="text-sm text-muted-foreground">Your issue is on its way to {readers.toLocaleString("en-US")} readers.</span>
      </div>
    </div>
  );
}
