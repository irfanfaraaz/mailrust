import { axisVector, fitProjection, iso, type Flat, type Plan, type Point3, type Projection } from "../../iso/iso-kit";

export const WIDTH = 640;
export const HEIGHT = 270;
export const AZIMUTH = 52;
export const ELEVATION = 30;

// The bench: x runs along the belts (left to right), y toward the viewer, z up.
export const BENCH: Plan = { x: -222, y: -94, w: 454, d: 196, r: 10 };
export const BENCH_H = 10;
export const FOOT_H = 5;

// Belt tops sit at BELT_Z. Letters and cards stand on it.
export const BELT_Z = 10.5;
export const NEWS = { x0: -168, x1: 124, yc: -1, w: 34 };
export const CODE = { x0: -168, x1: 124, yc: -52, w: 26 };

// Gate over the newsletter belt. It drops from GATE_UP to GATE_DOWN.
export const GATE_X = -92;
export const GATE_DOWN_Z = 12.4;
export const GATE_H = 12;
export const GATE_TRAVEL = 22;

// Dial gauge and the knob.
export const DIAL = { x: 70, y: 66, z: 44, r: 21 };
export const KNOB = { x: 186, y: 72, r: 11, top: 24 };

export const MAX_BOUNCE = 4;

function corners({ x, y, w, d }: Plan, z0: number, z1: number): Point3[] {
  const out: Point3[] = [];
  for (const px of [x, x + w]) for (const py of [y, y + d]) for (const z of [z0, z1]) out.push([px, py, z]);
  return out;
}

export const P: Projection = fitProjection(
  [
    ...corners(BENCH, -BENCH_H - FOOT_H, 0),
    [-148, -14, 56],
    [GATE_X - 4, -28, 66],
    [GATE_X - 4, 26, 66],
    [DIAL.x, DIAL.y, DIAL.z + DIAL.r + 8],
    [KNOB.x, KNOB.y, 26],
  ],
  WIDTH,
  HEIGHT,
  { pad: 6, azimuth: AZIMUTH, elevation: ELEVATION },
);

export const at = (point: Point3): Flat => iso(point, P);

const ORIGIN = at([0, 0, 0]);
const minus = (a: Flat, b: Flat): Flat => [a[0] - b[0], a[1] - b[1]];
export const EX = minus(at([1, 0, 0]), ORIGIN);
export const EY = minus(at([0, 1, 0]), ORIGIN);
export const EZ = minus(at([0, 0, 1]), ORIGIN);

const fine = (value: number) => Math.round(value * 100) / 100;

// Screen translation for a world move.
export const shift = (x: number, y = 0, z = 0) => `translate(${fine(EX[0] * x + EY[0] * y + EZ[0] * z)} ${fine(EX[1] * x + EY[1] * y + EZ[1] * z)})`;

export const move = (by: Point3) => {
  const [dx, dy] = axisVector(P, by);
  return `translate(${fine(dx)} ${fine(dy)})`;
};
