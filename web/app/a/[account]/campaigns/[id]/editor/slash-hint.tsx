"use client";
import { useEffect, useRef, useSyncExternalStore } from "react";

// A hand-written "Try typing /" note over an empty editor. It shows until the writer first types something, once per browser.
const KEY = "mr_slash_hint_seen";
const CHANGED = "mr:slash-hint";
let fallback = false; // storage blocked
function subscribe(onChange: () => void) {
  window.addEventListener("storage", onChange);
  window.addEventListener(CHANGED, onChange);
  return () => {
    window.removeEventListener("storage", onChange);
    window.removeEventListener(CHANGED, onChange);
  };
}
function readSeen() {
  try {
    return fallback || localStorage.getItem(KEY) === "1";
  } catch {
    return fallback;
  }
}

export function SlashHint({ empty }: { empty: boolean }) {
  const seen = useSyncExternalStore(subscribe, readSeen, () => true);
  const shown = useRef(false);
  useEffect(() => {
    if (seen) return;
    if (empty) { shown.current = true; return; }
    if (!shown.current) return; // the draft already had text: the note never showed, so it is not used up
    try { localStorage.setItem(KEY, "1"); } catch { fallback = true; }
    window.dispatchEvent(new Event(CHANGED));
  }, [empty, seen]);
  if (seen || !empty) return null;
  return (
    <div aria-hidden className="slash-hint pointer-events-none absolute top-0 right-2 flex flex-col items-end font-heading text-sm font-semibold italic text-[color-mix(in_oklab,var(--tab-newsletter)_70%,var(--foreground))]">
      Try typing /
      <svg width="60" height="30" viewBox="0 0 60 30"><path pathLength={1} d="M56 4 C 40 2, 20 8, 10 24 M10 24 l0 -9 M10 24 l9 -3" /></svg>
    </div>
  );
}
