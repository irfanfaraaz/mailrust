"use client";
import { useEffect, useRef } from "react";

// Count up once per page load; later visits in the same tab show the number straight away.
let played = false;

/** Counts a formatted figure ("1,284", "98.6%") up from zero on the first Overview of a visit. */
export function CountUp({ value }: { value: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    const m = /^(\D*)([\d,]*\.?\d+)(.*)$/.exec(value);
    const el = ref.current;
    if (!m || !el || played || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const [, pre, num, post] = m;
    const end = Number(num.replace(/,/g, ""));
    const decimals = num.split(".")[1]?.length ?? 0;
    const fmt = new Intl.NumberFormat("en", { minimumFractionDigits: decimals, maximumFractionDigits: decimals, useGrouping: num.includes(",") });
    const t0 = performance.now();
    let raf = 0;
    const step = (t: number) => {
      const p = Math.min((t - t0) / 600, 1);
      el.textContent = pre + fmt.format(end * (1 - (1 - p) ** 3)) + post;
      if (p < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    const done = setTimeout(() => (played = true));
    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(done);
      el.textContent = value;
    };
  }, [value]);
  return <span ref={ref}>{value}</span>;
}
