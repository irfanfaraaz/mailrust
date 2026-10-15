"use client";

import { useEffect } from "react";
import { replay, useStill } from "./demos";

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Page-wide motion for the landing: scroll reveals, the how-it-works pulse, the feature count, the / key and a console note. Renders nothing. */
export function LandingMotion() {
  const still = useStill();

  useEffect(() => {
    const page = document.querySelector<HTMLElement>(".mr-landing");
    if (!page) return;
    const cleanups: (() => void)[] = [];
    const $$ = (s: string) => [...page.querySelectorAll<HTMLElement>(s)];

    // / anywhere jumps to the editor and rings its slash menu, like the real editor
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "/" || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.target instanceof Element && e.target.closest("input, textarea, select, [contenteditable]")) return;
      e.preventDefault();
      page.querySelector<HTMLButtonElement>("#tab-news")?.click();
      page.querySelector(".editor")?.scrollIntoView({ behavior: still ? "auto" : "smooth", block: "center" });
      replay(page.querySelector(".slash"), "ring");
    };
    addEventListener("keydown", onKey);
    cleanups.push(() => removeEventListener("keydown", onKey));

    if (!still) {
      // sections rise in; only elements still below the fold, so nothing on screen blinks out after load
      const fold = innerHeight;
      const els = $$("main section:not(.hero) .head, #doors .door > *, .plate, .catalog > .win, .note, .flow, .back, .run > .win, .need, .proof > .win, .faq, .final h2, .final p, .final .ctas").filter(
        (el) => el.getBoundingClientRect().top > fold,
      );
      for (const el of els) {
        const sibs = [...(el.parentElement?.children ?? [])];
        el.style.setProperty("--d", String(Math.min(sibs.indexOf(el), 6)));
        el.classList.add("rv");
      }
      const rio = new IntersectionObserver(
        (es) => es.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("in");
            rio.unobserve(e.target);
          }
        }),
        { rootMargin: "0px 0px -8% 0px" },
      );
      els.forEach((el) => rio.observe(el));
      cleanups.push(() => rio.disconnect());

      // the feature count rolls up to the real number
      const count = page.querySelector<HTMLElement>(".count b");
      if (count) {
        const total = Number(count.textContent);
        const cio = new IntersectionObserver(([e]) => {
          if (!e.isIntersecting) return;
          cio.disconnect();
          const t0 = performance.now();
          const step = (t: number) => {
            const k = Math.min(1, (t - t0) / 1000);
            count.textContent = String(Math.round(total * (1 - (1 - k) ** 4)));
            if (k < 1) requestAnimationFrame(step);
          };
          requestAnimationFrame(step);
        }, { threshold: 0.6 });
        cio.observe(count);
        cleanups.push(() => {
          cio.disconnect();
          count.textContent = String(total);
        });
      }

      // how it works: a dot travels each arrow and each box lights up, only while the diagram is on screen
      const flow = page.querySelector<HTMLElement>(".flow");
      if (flow) {
        let run = 0;
        const steps = [...flow.children] as HTMLElement[];
        const pulse = async (me: number) => {
          while (me === run) {
            for (const s of steps) {
              if (me !== run) return;
              if (s.classList.contains("arrow")) {
                const r = s.getBoundingClientRect();
                s.style.setProperty("--len", `${Math.max(r.width, r.height) - 8}px`);
                replay(s, "go");
                await wait(500);
              } else {
                const wins = s.classList.contains("win") ? [s] : [...s.querySelectorAll<HTMLElement>(".win")];
                wins.forEach((w) => w.classList.add("lit"));
                await wait(420);
                wins.forEach((w) => w.classList.remove("lit"));
              }
            }
            steps.forEach((s) => s.classList.remove("go"));
            await wait(1200);
          }
        };
        const fio = new IntersectionObserver(([e]) => {
          run++;
          if (e.isIntersecting) pulse(run);
        });
        fio.observe(flow);
        cleanups.push(() => {
          run++;
          fio.disconnect();
        });
      }
    }

    return () => cleanups.forEach((f) => f());
  }, [still]);

  // a note for anyone reading the page source in the console
  useEffect(() => {
    console.log("%cMailrust%c\nReading the source? Every line is public.\ngit clone https://github.com/irfanfaraaz/mailrust", "font:700 16px monospace;color:#c4632f", "font:12px monospace");
  }, []);

  return null;
}
