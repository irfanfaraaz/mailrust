"use client";

import { useEffect, useRef } from "react";
import { useStill } from "./demos";

const fmt = (n: number) => n.toLocaleString("en-US");
const W = 320;
const H = 132;
const LANE_U = 50;
const LANE_N = 98;
const X0 = 98;
const X1 = 268;
const WS = [100, 200, 400, 800, 1600, 3200, 6400, 10000];

/** The hero: a pixel drawing of the engine. Urgent mail passes the newsletter, which goes out in growing waves. */
export function HeroScene() {
  const still = useStill();
  const cv = useRef<HTMLCanvasElement>(null);
  const rUrgent = useRef<HTMLElement>(null);
  const rWave = useRef<HTMLElement>(null);
  const rSize = useRef<HTMLElement>(null);
  const rBounce = useRef<HTMLElement>(null);

  useEffect(() => {
    const canvas = cv.current;
    const cx = canvas?.getContext("2d");
    if (!canvas || !cx) return;
    let ink = "#000";
    let paper = "#fff";
    let solid = "#000";
    let font = "monospace";
    const colors = () => {
      const s = getComputedStyle(canvas);
      ink = s.getPropertyValue("--ink").trim();
      paper = s.getPropertyValue("--paper").trim();
      solid = s.getPropertyValue("--solid").trim();
      font = s.getPropertyValue("--font-silkscreen").trim() || "monospace";
    };
    colors();

    const px = (x: number, y: number, w = 1, h = 1) => cx.fillRect(x | 0, y | 0, w, h);
    const box = (x: number, y: number, w: number, h: number) => {
      px(x, y, w, 1);
      px(x, y + h - 1, w, 1);
      px(x, y, 1, h);
      px(x + w - 1, y, 1, h);
    };
    const dither = (x: number, y: number, w: number, h: number, step = 2) => {
      for (let j = 0; j < h; j++) for (let i = j % 2; i < w; i += step) px(x + i, y + j);
    };
    const flap = (x: number, y: number) => {
      for (let i = 0; i < 5; i++) {
        px(x + 1 + i, y + 1 + i);
        px(x + 9 - i, y + 1 + i);
      }
    };
    const envelope = (x: number, y: number, filled: boolean) => {
      cx.fillStyle = filled ? solid : paper;
      px(x, y, 11, 8);
      cx.fillStyle = filled ? paper : ink;
      if (!filled) box(x, y, 11, 8);
      flap(x, y);
      cx.fillStyle = ink;
    };
    const text = (s: string, x: number, y: number) => {
      cx.font = `8px ${font}`;
      cx.textBaseline = "top";
      cx.fillText(s, x, y);
    };
    const show = (el: HTMLElement | null, s: string) => {
      if (el) el.textContent = s;
    };

    let t = 0;
    let urgent: { x: number }[] = [];
    let wave: { x: number }[] = [];
    let waveNo = 0;
    let waveTimer = 40;
    let urgentTimer = 30;
    let sentU = 0;
    let delivered = 0;

    const spawnWave = () => {
      waveNo = (waveNo % WS.length) + 1;
      wave = Array.from({ length: Math.min(waveNo + 1, 7) }, (_, i) => ({ x: X0 - 4 - i * 15 }));
      show(rWave.current, `Wave ${waveNo} of ${WS.length}`);
      show(rSize.current, `${fmt(WS[waveNo - 1])} readers`);
      show(rBounce.current, `${(0.3 + ((waveNo * 37) % 5) / 10).toFixed(1)}%`);
    };

    const frame = () => {
      t++;
      cx.fillStyle = paper;
      cx.fillRect(0, 0, W, H);
      cx.fillStyle = ink;
      // engine
      box(10, 22, 80, 98);
      box(11, 23, 78, 96);
      for (let j = 26; j < 36; j += 2) px(14, j, 72, 1);
      cx.fillStyle = paper;
      px(24, 26, 52, 10);
      cx.fillStyle = ink;
      text("ENGINE", 27, 27);
      dither(16, 42, 68, 16, 2);
      dither(16, 90, 68, 16, 4);
      cx.fillStyle = paper;
      px(20, 46, 52, 9);
      px(20, 94, 54, 9);
      cx.fillStyle = ink;
      text("URGENT", 22, 47);
      text("NEWSLTR", 22, 95);
      // lanes with moving belt ticks
      for (const y of [LANE_U, LANE_N]) {
        px(X0 - 8, y + 10, X1 - X0 + 12, 1);
        const off = (t * (y === LANE_U ? 2 : 1)) % 6;
        for (let x = X0 - 8 + off; x < X1 + 4; x += 6) px(x, y + 12, 2, 1);
      }
      text("URGENT LANE", X0, LANE_U - 14);
      text(`WAVE ${waveNo} · ${fmt(WS[waveNo - 1] ?? 100)}`, X0, LANE_N - 14);
      // inbox
      box(276, 22, 36, 98);
      for (let j = 26; j < 36; j += 2) px(279, j, 30, 1);
      cx.fillStyle = paper;
      px(282, 26, 24, 10);
      cx.fillStyle = ink;
      text("IN", 287, 27);
      const stack = Math.min(14, Math.floor(delivered / 2));
      for (let i = 0; i < stack; i++) px(282, 112 - i * 5, 24, 3);
      // urgent letters: fast, solid
      if (--urgentTimer <= 0) {
        urgent.push({ x: X0 - 6 });
        urgentTimer = 70 + ((t * 13) % 60);
      }
      urgent.forEach((u) => {
        u.x += 2.2;
        envelope(u.x, LANE_U, true);
      });
      const doneU = urgent.filter((u) => u.x > X1).length;
      if (doneU) {
        sentU += doneU;
        delivered += doneU;
        show(rUrgent.current, `${fmt(sentU)} sent`);
      }
      urgent = urgent.filter((u) => u.x <= X1);
      // newsletter wave: a group of outlined letters, then a pause before the next wave
      if (wave.length) {
        wave.forEach((w) => {
          w.x += 0.9;
          if (w.x > X0 - 12) envelope(w.x, LANE_N, false);
        });
        const before = wave.length;
        wave = wave.filter((w) => w.x <= X1);
        delivered += before - wave.length;
        if (!wave.length) waveTimer = 70;
      } else if (--waveTimer <= 0) spawnWave();
      if (!wave.length && waveTimer > 0) text(`NEXT WAVE IN ${Math.ceil(waveTimer / 7)} MIN`, X0, LANE_N + 2);
    };

    let raf = 0;
    let visible = true;
    let io: IntersectionObserver | undefined;
    let cancelled = false;
    const loop = () => {
      frame();
      raf = visible ? requestAnimationFrame(loop) : 0;
    };
    // the theme toggle flips data-theme on the page wrapper
    const themed = new MutationObserver(() => {
      colors();
      if (still) frame();
    });
    const wrapper = canvas.closest(".mr-landing");
    if (wrapper) themed.observe(wrapper, { attributes: true, attributeFilter: ["data-theme"] });

    document.fonts.ready.then(() => {
      if (cancelled) return;
      colors();
      spawnWave();
      if (still) {
        for (let i = 0; i < 120; i++) frame();
        return;
      }
      io = new IntersectionObserver(([e]) => {
        visible = e.isIntersecting;
        if (visible && !raf) raf = requestAnimationFrame(loop);
      });
      io.observe(canvas);
    });

    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      io?.disconnect();
      themed.disconnect();
    };
  }, [still]);

  return (
    <div className="win stage">
      <div className="bar">
        <i className="x" />
        <span className="t">mailrust serve</span>
      </div>
      <canvas
        ref={cv}
        width={W}
        height={H}
        role="img"
        aria-label="Animation: urgent emails pass the newsletter queue in their own lane, while the newsletter goes out in growing waves."
      />
      <dl className="readout">
        <div>
          <dt>Urgent lane</dt>
          <dd ref={rUrgent}>0 sent</dd>
        </div>
        <div>
          <dt>Newsletter</dt>
          <dd ref={rWave}>Wave 1 of 8</dd>
        </div>
        <div>
          <dt>This wave</dt>
          <dd ref={rSize}>100 readers</dd>
        </div>
        <div>
          <dt>Bounce rate</dt>
          <dd ref={rBounce}>0.4%</dd>
        </div>
      </dl>
    </div>
  );
}
