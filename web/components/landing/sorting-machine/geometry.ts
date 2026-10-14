import {
  cameraOf,
  circleRing,
  extrude,
  insetPlan,
  knurl,
  lineOnTop,
  onTop,
  pathOf,
  planOutline,
  ring,
  sideMatrix,
  slabOf,
  topMatrix,
  topTicks,
  type Flat,
  type Plan,
  type Plane,
  type Point3,
  type SolidPaths,
} from "../../iso/iso-kit";
import { BELT_Z, BENCH, BENCH_H, CODE, DIAL, FOOT_H, GATE_DOWN_Z, GATE_H, GATE_X, KNOB, MAX_BOUNCE, NEWS, P, at } from "./view";

const join = (paths: string[]) => paths.join("");
const RAD = Math.PI / 180;
const slab = (plan: Plan, z: number, h: number, steps = 4, bevel = 0) => slabOf(plan, z, h, P, steps, bevel);
const seg = (a: Point3, b: Point3) => pathOf([at(a), at(b)]);
const box = (x: number, y: number, w: number, d: number, r = 0): Plan => ({ x, y, w, d, r });

// ---------------------------------------------------------------- a cylinder lying along y
function hull(points: Flat[]): Flat[] {
  const sorted = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (o: Flat, a: Flat, b: Flat) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const lower: Flat[] = [];
  for (const point of sorted) {
    while (lower.length >= 2 && cross(lower[lower.length - 2]!, lower[lower.length - 1]!, point) <= 0) lower.pop();
    lower.push(point);
  }
  const upper: Flat[] = [];
  for (const point of [...sorted].reverse()) {
    while (upper.length >= 2 && cross(upper[upper.length - 2]!, upper[upper.length - 1]!, point) <= 0) upper.pop();
    upper.push(point);
  }
  return [...lower.slice(0, -1), ...upper.slice(0, -1)];
}

export function cylY(cx: number, cz: number, r: number, y0: number, y1: number, steps = 32): SolidPaths {
  const { sinA, cosA, sinE, cosE } = cameraOf(P);
  const view: Point3 = [cosA * cosE, sinA * cosE, sinE];
  const on = (angle: number, y: number): Point3 => [cx + r * Math.cos(angle), y, cz + r * Math.sin(angle)];
  const back = Array.from({ length: steps }, (_, i) => at(on((i / steps) * Math.PI * 2, y0)));
  const front = Array.from({ length: steps }, (_, i) => at(on((i / steps) * Math.PI * 2, y1)));
  const buckets: string[][] = [[], [], [], []];
  for (let i = 0; i < steps; i++) {
    const mid = ((i + 0.5) / steps) * Math.PI * 2;
    const nx = Math.cos(mid);
    const nz = Math.sin(mid);
    if (nx * view[0] + nz * view[2] <= 1e-9) continue;
    const lit = (-nx * sinA * 0.9 + nz * 0.5 + 1) / 2;
    const shade = Math.min(3, Math.max(0, Math.floor(lit * 4)));
    const j = (i + 1) % steps;
    buckets[shade]!.push(pathOf([back[i]!, back[j]!, front[j]!, front[i]!], true));
  }
  const cap = pathOf(front, true);
  const body = pathOf(hull([...back, ...front]), true);
  return { fill: body, outline: body + cap, crease: "", top: cap, shades: buckets.map(join) };
}

// A point on the plane y = face, u across (world x) and w up (world z) from a centre.
const faceAt = (cx: number, face: number, cz: number) => (u: number, w: number) => at([cx + u, face, cz + w]);

// ---------------------------------------------------------------- the bench
const FEET: Plane[] = [
  [BENCH.x + 22, BENCH.y + 18],
  [BENCH.x + BENCH.w - 22, BENCH.y + 18],
  [BENCH.x + 22, BENCH.y + BENCH.d - 18],
  [BENCH.x + BENCH.w - 22, BENCH.y + BENCH.d - 18],
];
const BENCH_SCREWS: Plane[] = [
  [BENCH.x + 9, BENCH.y + 9],
  [BENCH.x + BENCH.w - 9, BENCH.y + 9],
  [BENCH.x + 9, BENCH.y + BENCH.d - 9],
  [BENCH.x + BENCH.w - 9, BENCH.y + BENCH.d - 9],
];
const cyl = (cx: number, cy: number, r: number, z: number, h: number, steps = 28) => extrude(circleRing(cx, cy, r, steps), z, h, P, { convex: true });

const PLATE = box(-208, 62, 72, 17, 2);
const RULER = box(-208, 87, 190, 7, 1.5);
const RULER_FROM = -204;
const RULER_TO = -22;
const ruler = topTicks(RULER_FROM, RULER_TO, 4, 5, RULER.y + 0.8, 0.8, [1.8, 3.2], P);

export const BENCH_ART = {
  halo: slab(BENCH, -BENCH_H - FOOT_H, BENCH_H).fill,
  feet: FEET.map(([x, y]) => [cyl(x, y, 8, -BENCH_H - FOOT_H, 1.8), cyl(x, y, 6, -BENCH_H - FOOT_H + 1.8, FOOT_H - 1.8)]),
  slab: slab(BENCH, -BENCH_H, BENCH_H, 8, 1.6),
  groove: planOutline(insetPlan(BENCH, 4.8), 0, P),
  screws: BENCH_SCREWS.map(([x, y]) => at([x, y, 0])),
  screwRings: join(BENCH_SCREWS.map(([x, y]) => ring(x, y, 2.3, 0, P, 16))),
  plate: slab(PLATE, 0, 0.8),
  plateLine: planOutline(insetPlan(PLATE, 1.5), 0.8, P),
  plateScrews: [at([PLATE.x + 3.4, PLATE.y + PLATE.d / 2, 0.8]), at([PLATE.x + PLATE.w - 3.4, PLATE.y + PLATE.d / 2, 0.8])],
  plateRule: lineOnTop([PLATE.x + 8, PLATE.y + 12.2], [PLATE.x + PLATE.w - 8, PLATE.y + 12.2], 0.8, P),
  label: topMatrix([PLATE.x + 9, PLATE.y + 9.2], 0.8, P),
  ruler: slab(RULER, 0, 0.8, 4),
  rulerBase: lineOnTop([RULER_FROM, RULER.y + 0.8], [RULER_TO, RULER.y + 0.8], 0.8, P),
  rulerMinor: ruler.minor,
  rulerMajor: ruler.major,
};

// ---------------------------------------------------------------- a lane: bed, belt, rails, rollers
interface LaneSpec {
  x0: number;
  x1: number;
  yc: number;
  w: number;
}

const ROLLER_R = 4.5;
const ROLLER_Z = BELT_Z - ROLLER_R;
const BED_TOP = 8;

function laneOf({ x0, x1, yc, w }: LaneSpec) {
  const ya = yc - w / 2;
  const yb = yc + w / 2;
  const belt = box(x0, ya, x1 - x0, w, 2);
  const bed = box(x0 - 3, ya - 2, x1 - x0 + 6, w + 4, 1.5);
  const railBack = box(x0 + 9, ya - 4.8, x1 - x0 - 18, 2.8, 0.8);
  const railFront = box(x0 + 9, yb + 2, x1 - x0 - 18, 2.8, 0.8);
  const crossAt = [0.12, 0.38, 0.62, 0.88].map((share) => x0 + (x1 - x0) * share);
  const screwsAt = (rail: Plan) => {
    const out: Flat[] = [];
    for (let x = x0 + 16; x <= x1 - 12; x += 30) out.push(at([x, rail.y + rail.d / 2, 13.8]));
    return out;
  };
  const rollerAt = (x: number) => {
    const y0 = ya - 3;
    const y1 = yb + 3;
    return {
      x,
      y1,
      body: cylY(x, ROLLER_Z, ROLLER_R, y0, y1),
      axle: cylY(x, ROLLER_Z, 1.5, y1, y1 + 3.4, 12),
      hub: at([x, y1 + 3.4, ROLLER_Z]),
    };
  };
  return {
    spec: { x0, x1, yc, w, ya, yb },
    bed: slab(bed, 2, BED_TOP - 2, 4, 0.6),
    crosses: crossAt.map((x) => slab(box(x - 4, ya - 2, 8, w + 4, 1), 0, 2, 3)),
    belt: slab(belt, BED_TOP, BELT_Z - BED_TOP, 5, 0.5),
    edge: planOutline(insetPlan(belt, 1.6), BELT_Z, P, 5),
    railBack: slab(railBack, 2, 12, 3, 0.5),
    railFront: slab(railFront, 2, 12, 3, 0.5),
    railBackScrews: screwsAt(railBack),
    railFrontScrews: screwsAt(railFront),
    rollers: [rollerAt(x0), rollerAt(x1)],
  };
}

export const NEWS_LANE = laneOf(NEWS);
export const CODE_LANE = laneOf(CODE);

// Slats: short lines across the belt top that slide with the belt.
export const SLAT_PITCH = 14;
export function slatPath(lane: { x0: number; x1: number; ya: number; yb: number }, travel: number) {
  const out: string[] = [];
  const first = lane.x0 + ROLLER_R + 1.5;
  const last = lane.x1 - ROLLER_R - 1.5;
  const shiftBy = ((travel % SLAT_PITCH) + SLAT_PITCH) % SLAT_PITCH;
  for (let x = first + shiftBy - SLAT_PITCH; x <= last; x += SLAT_PITCH) {
    if (x < first) continue;
    out.push(seg([x, lane.ya + 1.6, BELT_Z], [x, lane.yb - 1.6, BELT_Z]));
  }
  return join(out);
}

// The mark on a roller's end cap turns with the belt.
export function spokePath(roller: { x: number; y1: number }, angle: number) {
  const radius = ROLLER_R - 0.2;
  return seg([roller.x, roller.y1, ROLLER_Z], [roller.x + radius * Math.cos(angle), roller.y1, ROLLER_Z + radius * Math.sin(angle)]);
}

// ---------------------------------------------------------------- letters and cards, built around the origin
const LW = 24;
const LD = 15;
const LH = 1.1;
const SEAL = 2.4;
const SEAL_AT: Plane = [0, 0.4];
const corner = (sx: number): Plane => [sx * (LW / 2 - 0.35), -LD / 2 + 0.35];

function toward(from: Plane, target: Plane, gap: number): Plane {
  const dx = target[0] - from[0];
  const dy = target[1] - from[1];
  const length = Math.hypot(dx, dy);
  return [target[0] - (dx / length) * gap, target[1] - (dy / length) * gap];
}

export function letterAt(cx: number, cy: number, z: number) {
  const left = corner(-1);
  const right = corner(1);
  const top = z + LH;
  const o = (p: Plane): Plane => [cx + p[0], cy + p[1]];
  const centre = o(SEAL_AT);
  return {
    body: slab(box(cx - LW / 2, cy - LD / 2, LW, LD, 0.9), z, LH, 3, 0.3),
    flap: join([
      onTop([o(left), toward(o(left), centre, SEAL)], top, P),
      onTop([o(right), toward(o(right), centre, SEAL)], top, P),
    ]),
    seal: ring(centre[0], centre[1], SEAL, top, P, 20),
    rules: join([lineOnTop(o([-9, 3.4]), o([-2, 3.4]), top, P), lineOnTop(o([-9, 5.4]), o([-4.5, 5.4]), top, P)]),
    stamp: onTop([o([6, 2.2]), o([10, 2.2]), o([10, 6]), o([6, 6])], top, P, true),
  };
}

export const LETTER = letterAt(0, 0, 0);

export function cardAt(cx: number, cy: number, z: number) {
  const top = z + 1;
  const dots: Plane[] = [];
  for (const row of [-0.4, 2.4]) for (const col of [-3, 0, 3]) dots.push([cx + col, cy + row]);
  return {
    body: slab(box(cx - 6.5, cy - 6.5, 13, 13, 1.6), z, 1, 3, 0.25),
    rule: lineOnTop([cx - 4.5, cy - 3.6], [cx + 4.5, cy - 3.6], top, P),
    dots: dots.map(([x, y]) => at([x, y, top])),
  };
}

export const CARD = cardAt(0, 0, 0);

// ---------------------------------------------------------------- hopper and dispenser (a box standing on the belt)
interface HopperSpec {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
  h: number;
  slot: number;
  wall: number;
}

function hopperOf({ x0, x1, y0, y1, h, slot, wall }: HopperSpec, stack: (z: number) => { body: SolidPaths; flap?: string; seal?: string }[]) {
  const top = BELT_Z + h;
  const slotTop = BELT_Z + slot;
  const w = x1 - x0;
  const d = y1 - y0;
  const lip = 1.4;
  const backWall = box(x0, y0, w, wall);
  const leftWall = box(x0, y0, wall, d);
  const rightWall = box(x1 - wall, y0, wall, d);
  const frontWall = box(x0, y1 - wall, w, wall);
  const feet = [
    box(x0 - 4, y0 - 4, 7, 7, 1),
    box(x1 - 3, y0 - 4, 7, 7, 1),
    box(x0 - 4, y1 - 3, 7, 7, 1),
    box(x1 - 3, y1 - 3, 7, 7, 1),
  ];
  return {
    feet: feet.map((plan) => slab(plan, BED_TOP, 1.8, 3)),
    footScrews: feet.map((plan) => at([plan.x + plan.w / 2, plan.y + plan.d / 2, BED_TOP + 1.8])),
    backWall: slab(backWall, BELT_Z, top - BELT_Z, 2, 0.4),
    leftWall: slab(leftWall, BELT_Z, top - BELT_Z, 2, 0.4),
    backLip: slab(box(x0 - lip, y0 - lip, w + lip * 2, wall + lip, 0.5), top - 1.6, 1.6, 2),
    leftLip: slab(box(x0 - lip, y0 - lip, wall + lip, d + lip * 2, 0.5), top - 1.6, 1.6, 2),
    stack: stack(BELT_Z + 1.2),
    rightWall: slab(rightWall, slotTop, top - slotTop, 2, 0.4),
    frontWall: slab(frontWall, BELT_Z, top - BELT_Z, 2, 0.4),
    rightLip: slab(box(x1 - wall, y0 - lip, wall + lip, d + lip * 2, 0.5), top - 1.6, 1.6, 2),
    frontLip: slab(box(x0 - lip, y1 - wall, w + lip * 2, wall + lip, 0.5), top - 1.6, 1.6, 2),
    // Edge-to-edge seams on the two faces that look at us.
    frontSeam: seg([x0, y1, BELT_Z + h * 0.42], [x1, y1, BELT_Z + h * 0.42]),
    sideSeam: seg([x1, y0, slotTop + 2.2], [x1, y1, slotTop + 2.2]),
    slotLine: seg([x1, y0 + 1.4, slotTop], [x1, y1 - 1.4, slotTop]),
    rivets: [0.18, 0.5, 0.82].map((share) => at([x0 + w * share, y1, BELT_Z + h * 0.21])),
    sideRivets: [0.2, 0.5, 0.8].map((share) => at([x1, y0 + d * share, BELT_Z + h * 0.62])),
    window: onTop([[x0 + w * 0.28, y1 + 0.01], [x0 + w * 0.72, y1 + 0.01]], top, P),
  };
}

const NEWS_HOPPER_SPEC: HopperSpec = { x0: -148, x1: -116, y0: -14, y1: 12, h: 18, slot: 3.2, wall: 2 };
export const NEWS_HOPPER = hopperOf(NEWS_HOPPER_SPEC, (z0) => {
  const jitter: Plane[] = [[0.5, -0.4], [-0.6, 0.3], [0.3, 0.5], [-0.3, -0.5], [0.6, 0.2], [-0.4, 0.4], [0.2, -0.3], [-0.5, -0.2]];
  return jitter.map(([jx, jy], index) => {
    const letter = letterAt(-132 + jx, -1 + jy, z0 + index * 1.35);
    return { body: letter.body, flap: index === jitter.length - 1 ? letter.flap : undefined, seal: index === jitter.length - 1 ? letter.seal : undefined };
  });
});

const CODE_HOPPER_SPEC: HopperSpec = { x0: -168, x1: -146, y0: -61, y1: -43, h: 12, slot: 2.6, wall: 1.8 };
export const CODE_HOPPER = hopperOf(CODE_HOPPER_SPEC, (z0) => {
  const jitter: Plane[] = [[0.4, 0.3], [-0.4, -0.3], [0.3, -0.4], [-0.3, 0.4]];
  return jitter.map(([jx, jy], index) => ({ body: cardAt(-156 + jx, -52 + jy, z0 + index * 1.25).body }));
});

// ---------------------------------------------------------------- crates at the belt ends
interface CrateSpec {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
  h: number;
}

function crateOf({ x0, x1, y0, y1, h }: CrateSpec) {
  const t = 2;
  const w = x1 - x0;
  const d = y1 - y0;
  const cx = (x0 + x1) / 2;
  const handle = (): string => {
    const a = cx - 7;
    const b = cx + 7;
    const zt = h - 4;
    const zb = h - 8;
    return pathOf([at([a + 1.4, y1, zt]), at([b - 1.4, y1, zt]), at([b, y1, zt - 1.4]), at([b, y1, zb + 1.4]), at([b - 1.4, y1, zb]), at([a + 1.4, y1, zb]), at([a, y1, zb + 1.4]), at([a, y1, zt - 1.4])], true);
  };
  return {
    floor: slab(box(x0, y0, w, d, 1), 0, 2, 3, 0.3),
    backWall: slab(box(x0, y0, w, t), 2, h - 2, 2, 0.4),
    rightWall: slab(box(x1 - t, y0, t, d), 2, h - 2, 2, 0.4),
    frontWall: slab(box(x0, y1 - t, w, t), 2, h - 2, 2, 0.4),
    handle: handle(),
    rivets: [x0 + 5, x1 - 5].map((x) => at([x, y1, h - 6])),
  };
}

export const NEWS_CRATE = crateOf({ x0: 130, x1: 176, y0: -22, y1: 20, h: 17 });
export const CODE_CRATE = crateOf({ x0: 130, x1: 160, y0: -65, y1: -39, h: 13 });

// ---------------------------------------------------------------- the gate over the newsletter belt
const POST_Y_BACK = NEWS.yc - NEWS.w / 2 - 12;
const POST_Y_FRONT = NEWS.yc + NEWS.w / 2 + 6;
const POST_TOP = 64;
const BAR_X = GATE_X - 3;
const BAR_Y0 = POST_Y_BACK + 5;
const BAR_Y1 = POST_Y_FRONT;
const posts = [box(GATE_X - 5, POST_Y_BACK, 6, 5, 0.8), box(GATE_X - 5, POST_Y_FRONT, 6, 5, 0.8)];

export const GATE_ART = {
  feet: posts.map((plan) => slab(insetPlan(plan, -2), 0, 1.6, 3)),
  footScrews: posts.map((plan) => at([plan.x + plan.w / 2, plan.y + plan.d / 2, 1.6])),
  posts: posts.map((plan) => slab(plan, 1.6, POST_TOP - 1.6, 3, 0.4)),
  beam: slab(box(GATE_X - 5, POST_Y_BACK, 6, POST_Y_FRONT + 5 - POST_Y_BACK, 0.8), POST_TOP - 6, 6, 3, 0.4),
  slotLines: posts.map((plan) => seg([plan.x + plan.w, plan.y + (plan.y < 0 ? plan.d : 0), 6], [plan.x + plan.w, plan.y + (plan.y < 0 ? plan.d : 0), POST_TOP - 7])).join(""),
  beamScrews: [0.2, 0.5, 0.8].map((share) => at([GATE_X + 1, POST_Y_BACK + (POST_Y_FRONT + 5 - POST_Y_BACK) * share, POST_TOP - 3])),
};

// The bar, drawn at its lowest position. The live code lifts it.
const BAR_LEN = BAR_Y1 - BAR_Y0;
const BAR_PLAN = box(BAR_X, BAR_Y0, 3, BAR_LEN, 0.6);
const BAR_TOP = GATE_DOWN_Z + GATE_H;
const face = BAR_X + 3;
const stamp = {
  y0: BAR_Y1 - 2.4,
  y1: BAR_Y0 + 2.4,
  z0: GATE_DOWN_Z + 2,
  z1: BAR_TOP - 2,
};

export const BAR_ART = {
  bar: slab(BAR_PLAN, GATE_DOWN_Z, GATE_H, 2, 0.4),
  halo: slab(BAR_PLAN, GATE_DOWN_Z, GATE_H).fill,
  frame: pathOf(
    [at([face + 0.02, stamp.y0, stamp.z0]), at([face + 0.02, stamp.y1, stamp.z0]), at([face + 0.02, stamp.y1, stamp.z1]), at([face + 0.02, stamp.y0, stamp.z1])],
    true,
  ),
  inner: pathOf(
    [
      at([face + 0.02, stamp.y0 - 1, stamp.z0 + 1]),
      at([face + 0.02, stamp.y1 + 1, stamp.z0 + 1]),
      at([face + 0.02, stamp.y1 + 1, stamp.z1 - 1]),
      at([face + 0.02, stamp.y0 - 1, stamp.z1 - 1]),
    ],
    true,
  ),
  text: sideMatrix([face + 0.02, stamp.y0 - 3], stamp.z1 - 1.6, P, "right"),
  textLength: stamp.y0 - stamp.y1 - 6,
  textSize: 5.4,
  textBase: 5.2,
  pins: [BAR_Y0 + 2.2, BAR_Y1 - 2.2].map((y) => at([face, y, GATE_DOWN_Z + GATE_H / 2])),
};

// ---------------------------------------------------------------- the dial gauge
const DIAL_FACE = DIAL.y + 5;
const dialPoint = faceAt(DIAL.x, DIAL_FACE, DIAL.z);
const SWEEP = 270;
const FROM = -135;
export const dialAngle = (value: number) => (FROM + (value / MAX_BOUNCE) * SWEEP) * RAD;
const onDial = (radius: number, value: number, off = 0): Flat => {
  const a = dialAngle(value);
  return at([DIAL.x + radius * Math.sin(a), DIAL_FACE + off, DIAL.z + radius * Math.cos(a)]);
};

function dialTicks() {
  const minor: string[] = [];
  const major: string[] = [];
  const danger: string[] = [];
  const dangerMajor: string[] = [];
  for (let tenth = 0; tenth <= 40; tenth++) {
    const value = tenth / 10;
    const big = tenth % 10 === 0;
    const red = tenth >= 20;
    const inner = big ? 12.6 : tenth % 5 === 0 ? 14 : 15.2;
    const d = pathOf([onDial(inner, value), onDial(16.8, value)]);
    (red ? (big ? dangerMajor : danger) : big ? major : minor).push(d);
  }
  return { minor: join(minor), major: join(major), danger: join(danger), dangerMajor: join(dangerMajor) };
}

function dialArc(from: number, to: number, radius: number) {
  const points: Flat[] = [];
  for (let step = 0; step <= 40; step++) points.push(onDial(radius, from + ((to - from) * step) / 40));
  return pathOf(points);
}

const DIAL_BASE = box(DIAL.x - 22, DIAL.y - 8, 44, 24, 3);
const COLUMN = box(DIAL.x - 4, DIAL.y - 5, 8, 8, 1);
const tk = dialTicks();

export const DIAL_ART = {
  base: slab(DIAL_BASE, 0, 4, 4, 0.6),
  baseScrews: [DIAL_BASE.x + 5, DIAL_BASE.x + DIAL_BASE.w - 5].map((x) => at([x, DIAL_BASE.y + DIAL_BASE.d - 5, 4])),
  baseLine: planOutline(insetPlan(DIAL_BASE, 2.8), 4, P, 4),
  column: slab(COLUMN, 4, DIAL.z - DIAL.r - 4 + 1.5, 3, 0.3),
  collar: slab(box(DIAL.x - 6.5, DIAL.y - 7.5, 13, 11, 1.5), 4, 3, 3, 0.3),
  body: cylY(DIAL.x, DIAL.z, DIAL.r, DIAL.y - 5, DIAL_FACE, 40),
  bezelRing: pathOf(Array.from({ length: 48 }, (_, i) => dialPoint(18.6 * Math.sin((i / 48) * Math.PI * 2), 18.6 * Math.cos((i / 48) * Math.PI * 2))), true),
  faceRing: pathOf(Array.from({ length: 48 }, (_, i) => dialPoint(17.6 * Math.sin((i / 48) * Math.PI * 2), 17.6 * Math.cos((i / 48) * Math.PI * 2))), true),
  minor: tk.minor,
  major: tk.major,
  danger: tk.danger,
  dangerMajor: tk.dangerMajor,
  arc: dialArc(2, 4, 16.8),
  arcSafe: dialArc(0, 2, 16.8),
  bezelScrews: [-1, 1].map((s) => dialPoint(s * 19.8 * Math.SQRT1_2, 19.8 * Math.SQRT1_2)),
  hub: cylY(DIAL.x, DIAL.z, 2.6, DIAL_FACE, DIAL_FACE + 3, 16),
  hubDot: dialPoint(0, 0),
  crown: cyl(DIAL.x + 0, DIAL.y - 1, 3.4, DIAL.z + DIAL.r - 0.5, 4.5, 20),
  crownKnurl: knurl(DIAL.x, DIAL.y - 1, 3.4, DIAL.z + DIAL.r - 0.5, DIAL.z + DIAL.r + 4, 14, P, 0.5),
};

export function needlePath(value: number) {
  const a = dialAngle(value);
  const tipR = 16.2;
  const tailR = -5;
  const half = 0.9;
  const s = Math.sin(a);
  const c = Math.cos(a);
  const at2 = (r: number, side: number): Flat => at([DIAL.x + r * s + side * c, DIAL_FACE + 3.2, DIAL.z + r * c - side * s]);
  return pathOf([at2(tipR, 0), at2(1, half), at2(tailR, half * 0.9), at2(tailR, -half * 0.9), at2(1, -half)], true);
}

// ---------------------------------------------------------------- the control box and the knob
const BOX = box(KNOB.x - 34, KNOB.y - 27, 68, 54, 4);
const BOX_TOP = 12;
const SCALE_R = 22;
const knobAngle = (value: number) => (180 + (value / MAX_BOUNCE) * 270) * RAD;
const onKnob = (radius: number, value: number): Plane => {
  const a = knobAngle(value);
  return [KNOB.x + radius * Math.sin(a), KNOB.y - radius * Math.cos(a)];
};

function knobTicks() {
  const minor: string[] = [];
  const major: string[] = [];
  const danger: string[] = [];
  const dangerMajor: string[] = [];
  for (let tenth = 0; tenth <= 40; tenth++) {
    const value = tenth / 10;
    const big = tenth % 10 === 0;
    const length = big ? 4.2 : tenth % 5 === 0 ? 3 : 1.8;
    const d = pathOf([at([...onKnob(SCALE_R - length, value), BOX_TOP]), at([...onKnob(SCALE_R, value), BOX_TOP])]);
    (tenth >= 20 ? (big ? dangerMajor : danger) : big ? major : minor).push(d);
  }
  return { minor: join(minor), major: join(major), danger: join(danger), dangerMajor: join(dangerMajor) };
}

const kt = knobTicks();

export const KNOB_ART = {
  box: slab(BOX, 0, BOX_TOP, 6, 1.2),
  inset: planOutline(insetPlan(BOX, 3.2), BOX_TOP, P, 6),
  screws: [BOX.x + 6, BOX.x + BOX.w - 6].flatMap((x) => [BOX.y + 6, BOX.y + BOX.d - 6].map((y) => at([x, y, BOX_TOP]))),
  minor: kt.minor,
  major: kt.major,
  danger: kt.danger,
  dangerMajor: kt.dangerMajor,
  arc: (() => {
    const points: Flat[] = [];
    for (let step = 0; step <= 40; step++) points.push(at([...onKnob(SCALE_R, 2 + (2 * step) / 40), BOX_TOP]));
    return pathOf(points);
  })(),
  arcSafe: (() => {
    const points: Flat[] = [];
    for (let step = 0; step <= 40; step++) points.push(at([...onKnob(SCALE_R, (2 * step) / 40), BOX_TOP]));
    return pathOf(points);
  })(),
  collar: cyl(KNOB.x, KNOB.y, KNOB.r + 3, BOX_TOP, 2, 32),
  shaft: cyl(KNOB.x, KNOB.y, KNOB.r, BOX_TOP + 2, KNOB.top - BOX_TOP - 2, 36),
  cap: extrude(circleRing(KNOB.x, KNOB.y, KNOB.r, 36), KNOB.top - 0.01, 0.01, P, { convex: true }),
  knurl: knurl(KNOB.x, KNOB.y, KNOB.r, BOX_TOP + 2, KNOB.top, 30, P, 0.6),
  capRing: ring(KNOB.x, KNOB.y, KNOB.r - 2.6, KNOB.top, P, 28),
  hit: at([KNOB.x, KNOB.y, KNOB.top]),
};

export function knobIndex(value: number) {
  const a = knobAngle(value);
  const from: Plane = [KNOB.x + 2.6 * Math.sin(a), KNOB.y - 2.6 * Math.cos(a)];
  const to: Plane = [KNOB.x + (KNOB.r - 0.9) * Math.sin(a), KNOB.y - (KNOB.r - 0.9) * Math.cos(a)];
  return onTop([from, to], KNOB.top, P);
}

// ---------------------------------------------------------------- tray of spare letters, and the cable
const TRAY = box(-114, 52, 48, 34, 3);
export const TRAY_ART = {
  plate: slab(TRAY, 0, 1.6, 4, 0.3),
  walls: [box(TRAY.x, TRAY.y, 2, TRAY.d), box(TRAY.x, TRAY.y, TRAY.w, 2), box(TRAY.x + TRAY.w - 2, TRAY.y, 2, TRAY.d)].map((plan) => slab(plan, 1.6, 7, 2, 0.3)),
  letters: [
    [0.6, 0.4],
    [-0.5, -0.3],
    [0.2, 0.6],
    [-0.4, 0.2],
    [0.5, -0.5],
  ].map(([jx, jy], index) => letterAt(TRAY.x + TRAY.w / 2 + jx!, TRAY.y + 18 + jy!, 1.6 + index * 1.3)),
  screws: [TRAY.x + 6, TRAY.x + TRAY.w - 6].map((x) => at([x, TRAY.y + TRAY.d - 4, 1.6])),
};

export const CABLE = {
  from: at([KNOB.x - 14, BOX.y + BOX.d, 6]),
  via: [at([KNOB.x - 14, BOX.y + BOX.d + 4, 0.8]), at([DIAL.x + 40, DIAL_BASE.y + DIAL_BASE.d + 8, 0.8]), at([DIAL.x + 10, DIAL_BASE.y + DIAL_BASE.d + 2, 0.8])],
  to: at([DIAL.x + 8, DIAL_BASE.y + DIAL_BASE.d, 2]),
};

// A small socket on the box front where the cable plugs in.
export const SOCKET = {
  plate: pathOf([at([KNOB.x - 18, BOX.y + BOX.d, 3]), at([KNOB.x - 10, BOX.y + BOX.d, 3]), at([KNOB.x - 10, BOX.y + BOX.d, 9]), at([KNOB.x - 18, BOX.y + BOX.d, 9])], true),
};

export const KNOB_HIT = ring(KNOB.x, KNOB.y, KNOB.r + 7, KNOB.top, P, 28);

export { BAR_TOP };
