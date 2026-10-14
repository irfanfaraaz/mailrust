import * as React from "react";
import { Dots, Line, Solid } from "../../iso/react/draw";
import {
  BAR_ART,
  BENCH_ART,
  CABLE,
  CARD,
  CODE_CRATE,
  CODE_HOPPER,
  CODE_LANE,
  DIAL_ART,
  GATE_ART,
  KNOB_ART,
  LETTER,
  NEWS_CRATE,
  NEWS_HOPPER,
  NEWS_LANE,
  SOCKET,
  TRAY_ART,
} from "./geometry";
import { Wire } from "../../iso/react/draw";

type Lane = typeof NEWS_LANE;
type Hopper = typeof NEWS_HOPPER;
type Crate = typeof NEWS_CRATE;

export const BENCH = (
  <g key="bench">
    <path className="iso-halo" d={BENCH_ART.halo} />
    {BENCH_ART.feet.map((parts, index) => (
      <g key={index}>
        {parts.map((paths, part) => (
          <Solid key={part} paths={paths} tone={part ? "mid" : "lo"} crease="faint" />
        ))}
      </g>
    ))}
    <Solid paths={BENCH_ART.slab} tone="mid" crease="faint" />
    <Line d={BENCH_ART.groove} tone="lo" />
    <Line d={BENCH_ART.screwRings} tone="lo" />
    <Dots points={BENCH_ART.screws} size={0.55} tone="mid" />
    <Solid paths={BENCH_ART.plate} tone="lo" crease="none" />
    <Line d={BENCH_ART.plateLine} tone="faint" />
    <Line d={BENCH_ART.plateRule} tone="lo" free />
    <Dots points={BENCH_ART.plateScrews} size={0.5} tone="mid" />
    <text className="iso-face-text" data-tone="hi" transform={BENCH_ART.label} fontSize={6.2}>
      MAILRUST
    </text>
    <Solid paths={BENCH_ART.ruler} tone="lo" crease="none" />
    <Line d={BENCH_ART.rulerBase} tone="lo" />
    <Line d={BENCH_ART.rulerMinor} tone="lo" />
    <Line d={BENCH_ART.rulerMajor} tone="mid" />
  </g>
);

export const TRAY = (
  <g key="tray">
    <Solid paths={TRAY_ART.plate} tone="mid" crease="faint" />
    <Solid paths={TRAY_ART.walls[0]!} tone="mid" crease="faint" />
    <Solid paths={TRAY_ART.walls[1]!} tone="mid" crease="faint" />
    {TRAY_ART.letters.map((letter, index) => (
      <g key={index} className="sm-paper">
        <Solid paths={letter.body} tone="hi" crease="faint" />
        {index === TRAY_ART.letters.length - 1 ? (
          <g>
            <Line d={letter.flap} tone="mid" />
            <path className="sm-seal" data-wave="3" d={letter.seal} />
            <Line d={letter.rules} tone="lo" free />
          </g>
        ) : null}
      </g>
    ))}
    <Solid paths={TRAY_ART.walls[2]!} tone="mid" crease="faint" />
    <Dots points={TRAY_ART.screws} size={0.5} tone="mid" />
  </g>
);

// A lane before the live parts: frame, bed, belt. The slats and rollers are live.
export const laneBase = (lane: Lane, key: string) => (
  <g key={key}>
    {lane.crosses.map((paths, index) => (
      <Solid key={index} paths={paths} tone="lo" crease="faint" />
    ))}
    <Solid paths={lane.bed} tone="mid" crease="faint" />
    <Solid paths={lane.railBack} tone="mid" crease="faint" />
    <Dots points={lane.railBackScrews} size={0.5} tone="mid" />
    <Solid paths={lane.belt} tone="lo" crease="faint" />
    <Line d={lane.edge} tone="faint" />
  </g>
);

export const laneRail = (lane: Lane, key: string) => (
  <g key={key}>
    <Solid paths={lane.railFront} tone="mid" crease="faint" />
    <Dots points={lane.railFrontScrews} size={0.5} tone="mid" />
  </g>
);

export const Roller = ({ roller, id }: { roller: Lane["rollers"][number]; id: string }) => (
  <g>
    <Solid paths={roller.body} tone="mid" crease="none" />
    <path className="iso-line" data-tone="lo" data-spoke={id} d="" />
    <Solid paths={roller.axle} tone="mid" crease="none" />
    <Dots points={[roller.hub]} size={0.5} tone="hi" />
  </g>
);

export const hopperBack = (h: Hopper, key: string) => (
  <g key={key}>
    {h.feet.map((paths, index) => (
      <Solid key={index} paths={paths} tone="lo" crease="faint" />
    ))}
    <Dots points={h.footScrews} size={0.45} tone="mid" />
    <Solid paths={h.backWall} tone="mid" crease="faint" />
    <Solid paths={h.leftWall} tone="mid" crease="faint" />
    <Solid paths={h.backLip} tone="mid" crease="faint" />
    <Solid paths={h.leftLip} tone="mid" crease="faint" />
  </g>
);

export const hopperStack = (h: Hopper, key: string, kind: "letter" | "card") => (
  <g key={key} className="sm-paper">
    {h.stack.map((item, index) => (
      <g key={index}>
        <Solid paths={item.body} tone={kind === "letter" ? "hi" : "hi"} crease="faint" />
        {item.flap ? <Line d={item.flap} tone="mid" /> : null}
        {item.seal ? <path className="sm-seal" data-wave="1" d={item.seal} /> : null}
      </g>
    ))}
  </g>
);

export const hopperFront = (h: Hopper, key: string) => (
  <g key={key}>
    <Solid paths={h.rightWall} tone="mid" crease="faint" />
    <Solid paths={h.frontWall} tone="mid" crease="faint" />
    <Solid paths={h.rightLip} tone="mid" crease="faint" />
    <Solid paths={h.frontLip} tone="mid" crease="faint" />
    <Line d={h.frontSeam} tone="faint" />
    <Line d={h.sideSeam} tone="faint" />
    <Line d={h.slotLine} tone="lo" />
    <Dots points={h.rivets} size={0.5} tone="mid" />
    <Dots points={h.sideRivets} size={0.5} tone="mid" />
  </g>
);

export const crateBack = (c: Crate, key: string) => (
  <g key={key}>
    <Solid paths={c.floor} tone="lo" crease="faint" />
    <Solid paths={c.backWall} tone="mid" crease="faint" />
  </g>
);

export const crateFront = (c: Crate, key: string) => (
  <g key={key}>
    <Solid paths={c.rightWall} tone="mid" crease="faint" />
    <Solid paths={c.frontWall} tone="mid" crease="faint" />
    <Line d={c.handle} tone="lo" />
    <Dots points={c.rivets} size={0.5} tone="mid" />
  </g>
);

export const LETTER_ART = (
  <g key="letter">
    <Solid paths={LETTER.body} tone="hi" crease="faint" />
    <Line d={LETTER.flap} tone="mid" />
    <path className="sm-seal" d={LETTER.seal} />
    <Line d={LETTER.rules} tone="lo" free />
    <Line d={LETTER.stamp} tone="lo" />
  </g>
);

export const CARD_ART = (
  <g key="card">
    <Solid paths={CARD.body} tone="hi" crease="faint" />
    <Line d={CARD.rule} tone="lo" free />
    <g className="sm-pips">
      <Dots points={CARD.dots} size={0.75} tone="hi" />
    </g>
  </g>
);

export const GATE_FRAME = (
  <g key="gate-frame" className="sm-primary">
    {GATE_ART.feet.map((paths, index) => (
      <Solid key={index} paths={paths} tone="lo" crease="faint" />
    ))}
    <Dots points={GATE_ART.footScrews} size={0.5} tone="mid" />
    {GATE_ART.posts.map((paths, index) => (
      <Solid key={index} paths={paths} tone="mid" crease="faint" />
    ))}
    <Line d={GATE_ART.slotLines} tone="lo" />
  </g>
);

export const GATE_BEAM = (
  <g key="gate-beam" className="sm-primary">
    <Solid paths={GATE_ART.beam} tone="mid" crease="faint" />
    <Dots points={GATE_ART.beamScrews} size={0.5} tone="mid" />
  </g>
);

// The bar is drawn at its lowest position; the stamp ink fades in as it lands.
export const BAR = (
  <>
    <path className="iso-halo sm-shadow" d={BAR_ART.halo} />
    <Solid paths={BAR_ART.bar} tone="hi" crease="faint" />
    <Dots points={BAR_ART.pins} size={0.5} tone="mid" />
    <g className="sm-stamp" data-stamp="">
      <Line d={BAR_ART.frame} tone="red" />
      <Line d={BAR_ART.inner} tone="red" />
      <text
        className="sm-stamp-text"
        transform={BAR_ART.text}
        y={BAR_ART.textBase}
        fontSize={BAR_ART.textSize}
        textLength={BAR_ART.textLength}
        lengthAdjust="spacingAndGlyphs"
      >
        PAUSED
      </text>
    </g>
  </>
);

export const DIAL = (
  <g key="dial">
    <Solid paths={DIAL_ART.base} tone="lo" crease="faint" />
    <Line d={DIAL_ART.baseLine} tone="faint" />
    <Dots points={DIAL_ART.baseScrews} size={0.5} tone="mid" />
    <Solid paths={DIAL_ART.column} tone="mid" crease="faint" />
    <Solid paths={DIAL_ART.collar} tone="mid" crease="faint" />
    <Solid paths={DIAL_ART.crown} tone="mid" crease="faint" />
    <Line d={DIAL_ART.crownKnurl} tone="lo" />
    <Solid paths={DIAL_ART.body} tone="mid" crease="none" />
    <Line d={DIAL_ART.bezelRing} tone="lo" />
    <Line d={DIAL_ART.faceRing} tone="faint" />
    <Line d={DIAL_ART.minor} tone="lo" />
    <Line d={DIAL_ART.major} tone="hi" />
    <Line d={DIAL_ART.danger} tone="red" />
    <Line d={DIAL_ART.dangerMajor} tone="red" />
    <Line d={DIAL_ART.arc} tone="red" />
    <Line d={DIAL_ART.arcSafe} tone="green" />
    <Dots points={DIAL_ART.bezelScrews} size={0.5} tone="mid" />
  </g>
);

export const HUB = (
  <g key="hub">
    <Solid paths={DIAL_ART.hub} tone="mid" crease="none" />
    <Dots points={[DIAL_ART.hubDot]} size={0.5} tone="hi" />
  </g>
);

export const CONTROL_BOX = (
  <g key="box">
    <Solid paths={KNOB_ART.box} tone="mid" crease="faint" />
    <Line d={KNOB_ART.inset} tone="faint" />
    <Dots points={KNOB_ART.screws} size={0.55} tone="mid" />
    <Line d={KNOB_ART.minor} tone="lo" />
    <Line d={KNOB_ART.major} tone="hi" />
    <Line d={KNOB_ART.danger} tone="red" />
    <Line d={KNOB_ART.dangerMajor} tone="red" />
    <Line d={KNOB_ART.arc} tone="red" />
    <Line d={KNOB_ART.arcSafe} tone="green" />
    <Line d={SOCKET.plate} tone="lo" />
  </g>
);

export const KNOB = (
  <g key="knob" className="sm-primary">
    <Solid paths={KNOB_ART.collar} tone="mid" crease="faint" />
    <Solid paths={KNOB_ART.shaft} tone="mid" crease="faint" />
    <Line d={KNOB_ART.knurl} tone="lo" />
    <Line d={KNOB_ART.capRing} tone="faint" />
  </g>
);

export const CABLE_ART = <Wire key="cable" from={CABLE.from} to={CABLE.to} via={CABLE.via} tone="mid" />;

export { CODE_CRATE, CODE_HOPPER, CODE_LANE, NEWS_CRATE, NEWS_HOPPER, NEWS_LANE };
