"use client";
import { XIcon } from "lucide-react";
import { useSyncExternalStore } from "react";
import { Stamp } from "./stamp";

export type Milestone = { id: string; badge: string; title: string; body: string };

const KEY = (account: string) => `mr_milestones_${account}`;

const CHANGED = "mr:milestones";

function subscribe(onChange: () => void) {
  window.addEventListener("storage", onChange);
  window.addEventListener(CHANGED, onChange);
  return () => {
    window.removeEventListener("storage", onChange);
    window.removeEventListener(CHANGED, onChange);
  };
}

// Used only when storage is blocked, so a closed note still closes for this page load.
const fallback = new Map<string, string>();

function readRaw(account: string): string {
  try {
    return localStorage.getItem(KEY(account)) ?? fallback.get(account) ?? "[]";
  } catch {
    return fallback.get(account) ?? "[]";
  }
}

/** A sticky note for a real first (first issue, 100 readers). Closing it also closes the smaller ones below it, for this browser. */
export function MilestoneNote({ account, milestones }: { account: string; milestones: Milestone[] }) {
  // The server has no closed list, so it renders nothing and a closed note never flashes.
  const raw = useSyncExternalStore(subscribe, () => readRaw(account), () => null);
  const closed: string[] = raw === null ? [] : JSON.parse(raw);
  const m = raw !== null && milestones.find((x) => !closed.includes(x.id));
  if (!m) return null;
  const close = () => {
    const next = [...closed, ...milestones.slice(milestones.indexOf(m)).map((x) => x.id)];
    try {
      localStorage.setItem(KEY(account), JSON.stringify(next));
    } catch {
      fallback.set(account, JSON.stringify(next)); // storage blocked: closed until the page reloads
    }
    window.dispatchEvent(new Event(CHANGED));
  };
  return (
    <aside aria-label="Milestone" className="note-pin flex max-w-xl -rotate-[0.6deg] items-start gap-3 rounded-[4px_18px_6px_14px/14px_6px_18px_4px] border-2 border-foreground bg-paper px-4 py-3.5 shadow-[3px_4px_0_var(--ink-shadow)]">
      <Stamp tone="good">{m.badge}</Stamp>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <p className="font-heading text-lg leading-snug font-semibold">{m.title}</p>
        <p className="text-sm text-muted-foreground">{m.body}</p>
      </div>
      <button type="button" onClick={close} aria-label="Close this note" className="-m-1.5 grid size-9 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-foreground/5 hover:text-foreground">
        <XIcon aria-hidden className="size-4" />
      </button>
    </aside>
  );
}
