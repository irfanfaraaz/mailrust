"use client";

import * as React from "react";
import { asksForStillness, follow, useReadout } from "../../iso/react/live";
import {
  BAR,
  BENCH,
  CABLE_ART,
  CARD_ART,
  CODE_CRATE,
  CODE_HOPPER,
  CODE_LANE,
  CONTROL_BOX,
  DIAL,
  GATE_BEAM,
  GATE_FRAME,
  HUB,
  KNOB,
  LETTER_ART,
  NEWS_CRATE,
  NEWS_HOPPER,
  NEWS_LANE,
  Roller,
  TRAY,
  crateBack,
  crateFront,
  hopperBack,
  hopperFront,
  hopperStack,
  laneBase,
  laneRail,
} from "./art";
import { KNOB_HIT, needlePath, knobIndex, slatPath, spokePath } from "./geometry";
import { CODE_SLOTS, MAX_TENTHS, NEWS_SLOTS, clampTenths, createSim, preroll, readoutOf, settle, speechOf, step, tourTenths } from "./model";
import { BELT_Z, CODE, GATE_TRAVEL, HEIGHT, NEWS, WIDTH, move, shift } from "./view";

const RESUME_AFTER = 12000; // ms of quiet before the tour takes the knob back
const TENTHS_PER_RADIAN = MAX_TENTHS / ((270 * Math.PI) / 180);
const ROLLER_R = 4.5;
const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

const pool = (count: number, kind: "news" | "code") =>
  Array.from({ length: count }, (_, index) => (
    <g key={index} className="sm-paper" data-piece={kind} visibility="hidden">
      {kind === "news" ? LETTER_ART : CARD_ART}
    </g>
  ));

const NEWS_POOL = pool(NEWS_SLOTS, "news");
const CODE_POOL = pool(CODE_SLOTS, "code");

const spokeAngle = (travel: number) => travel / ROLLER_R;

export const FIRST_READOUT = (() => {
  const sim = createSim(4);
  preroll(sim, 7.5);
  settle(sim);
  return readoutOf(sim);
})();

export function Machine() {
  const { anchor, say } = useReadout<HTMLDivElement>();

  React.useEffect(() => {
    const el = anchor.current;
    if (!el) return;
    const sim = createSim(4);
    preroll(sim, 7.5);
    settle(sim);

    const one = <T extends Element>(selector: string) => el.querySelector<T>(selector);
    const all = <T extends Element>(selector: string) => Array.from(el.querySelectorAll<T>(selector));
    const newsPieces = all<SVGGElement>('[data-piece="news"]');
    const codePieces = all<SVGGElement>('[data-piece="code"]');
    const gate = one<SVGGElement>("[data-gate]");
    const stamp = one<SVGGElement>("[data-stamp]");
    const needle = one<SVGPathElement>("[data-needle]");
    const index = one<SVGPathElement>("[data-index]");
    const slats = { news: one<SVGPathElement>('[data-slats="news"]'), code: one<SVGPathElement>('[data-slats="code"]') };
    const spokes = {
      n0: one<SVGPathElement>('[data-spoke="n0"]'),
      n1: one<SVGPathElement>('[data-spoke="n1"]'),
      c0: one<SVGPathElement>('[data-spoke="c0"]'),
      c1: one<SVGPathElement>('[data-spoke="c1"]'),
    };

    let shown = sim.tenths / 10; // where the knob index points
    let tourClock = 0;
    let lastTouch = -Infinity;
    let frame = 0;
    let last = 0;
    let visible = false;
    let spoken = "";
    const travelDrawn = { news: NaN, code: NaN };

    const place = (nodes: SVGGElement[], slots: (typeof sim.news.slots), y: number) => {
      nodes.forEach((node, slot) => {
        const piece = slots[slot];
        if (!piece) {
          if (node.getAttribute("visibility") !== "hidden") node.setAttribute("visibility", "hidden");
          return;
        }
        node.setAttribute("visibility", "visible");
        node.setAttribute("transform", move([piece.x, y, BELT_Z - piece.drop]));
        const wave = String(piece.wave);
        if (node.getAttribute("data-wave") !== wave) node.setAttribute("data-wave", wave);
      });
    };

    const draw = () => {
      const lift = (1 - sim.gate.at) * GATE_TRAVEL;
      gate?.setAttribute("transform", shift(0, 0, lift));
      if (stamp) stamp.style.opacity = String(clamp((sim.gate.at - 0.55) / 0.4, 0, 1));
      needle?.setAttribute("d", needlePath(clamp(sim.needle.at, -0.1, 4.1)));
      index?.setAttribute("d", knobIndex(shown));
      if (travelDrawn.news !== sim.news.travel) {
        slats.news?.setAttribute("d", slatPath(NEWS_LANE.spec, sim.news.travel));
        spokes.n0?.setAttribute("d", spokePath(NEWS_LANE.rollers[0]!, spokeAngle(sim.news.travel)));
        spokes.n1?.setAttribute("d", spokePath(NEWS_LANE.rollers[1]!, spokeAngle(sim.news.travel)));
        travelDrawn.news = sim.news.travel;
      }
      if (travelDrawn.code !== sim.code.travel) {
        slats.code?.setAttribute("d", slatPath(CODE_LANE.spec, sim.code.travel));
        spokes.c0?.setAttribute("d", spokePath(CODE_LANE.rollers[0]!, spokeAngle(sim.code.travel)));
        spokes.c1?.setAttribute("d", spokePath(CODE_LANE.rollers[1]!, spokeAngle(sim.code.travel)));
        travelDrawn.code = sim.code.travel;
      }
      place(newsPieces, sim.news.slots, NEWS.yc);
      place(codePieces, sim.code.slots, CODE.yc);
      say(readoutOf(sim));
      const words = speechOf(sim);
      if (words !== spoken) {
        spoken = words;
        el.setAttribute("aria-valuenow", String(sim.tenths / 10));
        el.setAttribute("aria-valuetext", words);
      }
    };

    const still = () => {
      settle(sim);
      shown = sim.tenths / 10;
      draw();
    };

    const tick = (now: number) => {
      const dt = last ? Math.min((now - last) / 1000, 1 / 30) : 1 / 60;
      last = now;
      if (asksForStillness()) {
        frame = 0;
        still();
        return;
      }
      tourClock += dt;
      if (now - lastTouch > RESUME_AFTER) sim.tenths = tourTenths(tourClock);
      shown = follow(shown, sim.tenths / 10, dt, 0.06);
      step(sim, dt);
      draw();
      frame = requestAnimationFrame(tick);
    };

    const kick = () => {
      if (frame || !visible) return;
      if (asksForStillness()) {
        still();
        return;
      }
      last = 0;
      frame = requestAnimationFrame(tick);
    };

    const stop = () => {
      cancelAnimationFrame(frame);
      frame = 0;
    };

    const commit = (tenths: number) => {
      sim.tenths = clampTenths(tenths);
      lastTouch = performance.now();
      if (asksForStillness()) still();
      else kick();
    };

    // Knob: drag around its centre, like a real one.
    let drag: { id: number; cx: number; cy: number; prev: number; value: number } | null = null;
    const angleOf = (event: PointerEvent, drag: { cx: number; cy: number }) => Math.atan2(event.clientY - drag.cy, event.clientX - drag.cx);

    const down = (event: PointerEvent) => {
      const hit = (event.target as Element).closest("[data-knob]");
      if (!hit) return;
      const box = hit.getBoundingClientRect();
      const start = { cx: box.left + box.width / 2, cy: box.top + box.height / 2 };
      drag = { id: event.pointerId, ...start, prev: angleOf(event, start), value: sim.tenths };
      hit.setPointerCapture(event.pointerId);
      el.setAttribute("data-held", "");
      el.focus({ preventScroll: true });
      event.preventDefault();
    };
    const moveKnob = (event: PointerEvent) => {
      if (!drag || event.pointerId !== drag.id) return;
      const next = angleOf(event, drag);
      let delta = next - drag.prev;
      if (delta > Math.PI) delta -= Math.PI * 2;
      if (delta < -Math.PI) delta += Math.PI * 2;
      drag.prev = next;
      drag.value = clamp(drag.value + delta * TENTHS_PER_RADIAN, 0, MAX_TENTHS);
      commit(drag.value);
    };
    const up = (event: PointerEvent) => {
      if (!drag || event.pointerId !== drag.id) return;
      drag = null;
      el.removeAttribute("data-held");
    };
    const keys = (event: KeyboardEvent) => {
      const big = event.shiftKey ? 5 : 1;
      const table: Record<string, number> = { ArrowUp: big, ArrowRight: big, ArrowDown: -big, ArrowLeft: -big, PageUp: 5, PageDown: -5 };
      let next: number | null = null;
      if (event.key in table) next = sim.tenths + table[event.key]!;
      else if (event.key === "Home") next = 0;
      else if (event.key === "End") next = MAX_TENTHS;
      if (next === null) return;
      event.preventDefault();
      commit(next);
    };

    el.addEventListener("pointerdown", down);
    el.addEventListener("pointermove", moveKnob);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
    el.addEventListener("keydown", keys);

    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const motionChanged = () => {
      stop();
      if (query.matches) still();
      else kick();
    };
    query.addEventListener("change", motionChanged);

    const watcher = new IntersectionObserver(
      (entries) => {
        visible = Boolean(entries.at(-1)?.isIntersecting);
        if (visible) kick();
        else stop();
      },
      { rootMargin: "120px 0px" },
    );
    watcher.observe(el);

    draw();
    el.setAttribute("data-ready", "");

    return () => {
      stop();
      watcher.disconnect();
      query.removeEventListener("change", motionChanged);
      el.removeEventListener("pointerdown", down);
      el.removeEventListener("pointermove", moveKnob);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      el.removeEventListener("keydown", keys);
    };
  }, [anchor, say]);

  return (
    <div
      ref={anchor}
      className="sm-root"
      role="slider"
      tabIndex={0}
      aria-label="Bounce rate of the account. Past 2 percent, sending pauses."
      aria-orientation="horizontal"
      aria-valuemin={0}
      aria-valuemax={4}
      aria-valuenow={0.4}
      aria-valuetext={FIRST_READOUT}
    >
      <svg className="iso-svg sm-svg" viewBox={`0 0 ${WIDTH} ${HEIGHT}`} aria-hidden="true" focusable="false">
        {BENCH}
        {TRAY}

        <g className="sm-code">
        {laneBase(CODE_LANE, "code-lane")}
        <path className="iso-line" data-tone="faint" data-slats="code" d="" />
        <Roller roller={CODE_LANE.rollers[0]!} id="c0" />
        <Roller roller={CODE_LANE.rollers[1]!} id="c1" />
        {laneRail(CODE_LANE, "code-rail")}
        {hopperBack(CODE_HOPPER, "code-hopper-back")}
        {hopperStack(CODE_HOPPER, "code-stack", "card")}
        {crateBack(CODE_CRATE, "code-crate-back")}
        {CODE_POOL}
        {hopperFront(CODE_HOPPER, "code-hopper-front")}
        {crateFront(CODE_CRATE, "code-crate-front")}
        </g>

        <g className="sm-news">
        {laneBase(NEWS_LANE, "news-lane")}
        <path className="iso-line" data-tone="faint" data-slats="news" d="" />
        <Roller roller={NEWS_LANE.rollers[0]!} id="n0" />
        <Roller roller={NEWS_LANE.rollers[1]!} id="n1" />
        {laneRail(NEWS_LANE, "news-rail")}
        {hopperBack(NEWS_HOPPER, "news-hopper-back")}
        {hopperStack(NEWS_HOPPER, "news-stack", "letter")}
        {crateBack(NEWS_CRATE, "news-crate-back")}
        {NEWS_POOL}
        {hopperFront(NEWS_HOPPER, "news-hopper-front")}
        {GATE_FRAME}
        <g data-gate="">{BAR}</g>
        {GATE_BEAM}
        {crateFront(NEWS_CRATE, "news-crate-front")}
        </g>

        {DIAL}
        <path className="sm-needle" data-needle="" d="" />
        {HUB}
        {CONTROL_BOX}
        {KNOB}
        <path className="iso-line" data-tone="lit" data-index="" d="" />
        <path data-knob="" d={KNOB_HIT} />
        {CABLE_ART}
      </svg>
    </div>
  );
}

