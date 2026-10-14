"use client";
import { XIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { Stamp } from "./stamp";

export type Milestone = { id: string; badge: string; title: string; body: string };

const KEY = (account: string) => `mr_milestones_${account}`;

function read(account: string): string[] {
  try {
    return JSON.parse(localStorage.getItem(KEY(account)) ?? "[]");
  } catch {
    return [];
  }
}

/** A sticky note for a real first (first issue, 100 readers). Closing it also closes the smaller ones below it, for this browser. */
export function MilestoneNote({ account, milestones }: { account: string; milestones: Milestone[] }) {
  const [closed, setClosed] = useState<string[] | null>(null);
  useEffect(() => setClosed(read(account)), [account]);
  // Before the closed list loads, show nothing, so a closed note never flashes.
  const m = closed && milestones.find((x) => !closed.includes(x.id));
  if (!m) return null;
  const close = () => {
    const next = [...closed, ...milestones.slice(milestones.indexOf(m)).map((x) => x.id)];
    setClosed(next);
    try {
      localStorage.setItem(KEY(account), JSON.stringify(next));
    } catch {
      /* storage blocked: the note stays closed until the page reloads */
    }
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
