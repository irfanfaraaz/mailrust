"use client";

import * as React from "react";

const STILLNESS = "(prefers-reduced-motion: reduce)";

export const asksForStillness = () => typeof window !== "undefined" && window.matchMedia(STILLNESS).matches;

export function useStillness() {
  return React.useSyncExternalStore(
    (changed) => {
      const query = window.matchMedia(STILLNESS);
      query.addEventListener("change", changed);
      return () => query.removeEventListener("change", changed);
    },
    asksForStillness,
    () => false,
  );
}

// The stage is a plain box. Figures watch their own visibility with a ref, so no re-render ever happens per frame.
export function Stage({ className, children }: { className: string; children: React.ReactNode }) {
  return <div className={className}>{children}</div>;
}

export function useReadout<T extends Element>() {
  const anchor = React.useRef<T>(null);
  const slot = React.useRef<Element | null>(null);
  const said = React.useRef<string | null>(null);
  React.useEffect(() => {
    slot.current = anchor.current?.closest(".iso-plate")?.querySelector("[data-readout]") ?? null;
  }, []);
  const say = React.useCallback((text: string) => {
    slot.current ??= anchor.current?.closest(".iso-plate")?.querySelector("[data-readout]") ?? null;
    if (!slot.current || text === said.current) return;
    said.current = text;
    slot.current.textContent = text;
  }, []);
  return { anchor, say };
}

export function follow(current: number, target: number, dt: number, seconds: number, calm = false) {
  if (calm) return target;
  const next = current + (target - current) * (1 - Math.exp(-dt / seconds));
  return Math.abs(target - next) < 1e-3 ? target : next;
}

export interface Spring {
  at: number;
  rate: number;
}

export function spring(state: Spring, target: number, dt: number, stiffness: number, damping: number, calm = false) {
  const drag = calm ? 2 * Math.sqrt(stiffness) : damping;
  const steps = Math.max(Math.ceil(dt * 240), 1);
  const slice = dt / steps;
  for (let step = 0; step < steps; step++) {
    state.rate += (target - state.at) * stiffness * slice - state.rate * drag * slice;
    state.at += state.rate * slice;
  }
  if (Math.abs(target - state.at) < 5e-4 && Math.abs(state.rate) < 5e-4) {
    state.at = target;
    state.rate = 0;
  }
  return state.at === target && state.rate === 0;
}
