// The machine's model. Pure numbers, no DOM. The same state moves the parts and writes the readout.
//
// What is true here, from the engine:
//  - A campaign goes out in waves, the most engaged readers first.
//  - Login and other transactional mail takes a faster lane than newsletters.
//  - An account that passes 2% bounces is paused automatically.
// Each letter stands for a batch, so "sent" counts letters, not messages.

export const MAX_TENTHS = 40; // bounce rate on the knob, in tenths of a percent
export const PAUSE_TENTHS = 20; // paused when the rate is past 2.0%
export const WAVE_SIZE = 5;
export const WAVES = 3;

export const NEWS_SPEED = 40; // units per second
export const CODE_SPEED = 120;
export const NEWS_START = -132;
export const CODE_START = -156;
export const LANE_END = 134; // a piece leaves the belt when its centre passes this x
export const NEWS_SLOTS = 14;
export const CODE_SLOTS = 5;

const SPACING = 30;
const WAVE_GAP = 96;
const CODE_SPACING = 150;
const FALL_G = 700;
export const FALL_DEPTH = 9;

export type Lane = "news" | "code";

export interface Piece {
  x: number;
  wave: number;
  falling: boolean;
  age: number;
  drop: number; // how far it has dropped below the belt
  vx: number;
}

interface Spring {
  at: number;
  rate: number;
}

interface LaneState {
  travel: number;
  nextAt: number;
  slots: (Piece | null)[];
}

export interface Sim {
  tenths: number;
  needle: Spring;
  gate: Spring; // 0 raised, 1 down on the belt
  run: number; // belt speed as a share of full speed
  news: LaneState;
  code: LaneState;
  inWave: number;
  wave: number;
  sent: number;
  clock: number;
}

const lane = (count: number): LaneState => ({ travel: 0, nextAt: 0, slots: Array.from({ length: count }, () => null) });

export function createSim(tenths = 4): Sim {
  return {
    tenths,
    needle: { at: tenths / 10, rate: 0 },
    gate: { at: 0, rate: 0 },
    run: 1,
    news: lane(NEWS_SLOTS),
    code: lane(CODE_SLOTS),
    inWave: 0,
    wave: 1,
    sent: 0,
    clock: 0,
  };
}

export const isPaused = (sim: Sim) => sim.tenths > PAUSE_TENTHS;

export const clampTenths = (value: number) => Math.min(MAX_TENTHS, Math.max(0, Math.round(value)));

function springTo(state: Spring, target: number, dt: number, stiffness: number, damping: number) {
  const steps = Math.max(Math.ceil(dt * 240), 1);
  const slice = dt / steps;
  for (let step = 0; step < steps; step++) {
    state.rate += (target - state.at) * stiffness * slice - state.rate * damping * slice;
    state.at += state.rate * slice;
  }
  if (Math.abs(target - state.at) < 5e-4 && Math.abs(state.rate) < 5e-4) {
    state.at = target;
    state.rate = 0;
  }
}

function stepGate(gate: Spring, down: boolean, dt: number) {
  if (down) {
    gate.rate += 44 * dt;
    gate.at += gate.rate * dt;
    if (gate.at >= 1) {
      gate.at = 1;
      gate.rate = gate.rate > 0.9 ? -gate.rate * 0.22 : 0;
    }
  } else {
    gate.rate = 0;
    gate.at += (0 - gate.at) * (1 - Math.exp(-dt / 0.2));
    if (gate.at < 1e-3) gate.at = 0;
  }
}

function advance(state: LaneState, speed: number, dt: number, onGone: () => void) {
  const distance = speed * dt;
  state.travel += distance;
  for (let index = 0; index < state.slots.length; index++) {
    const piece = state.slots[index];
    if (!piece) continue;
    if (!piece.falling) {
      piece.x += distance;
      if (piece.x >= LANE_END) {
        piece.falling = true;
        piece.age = 0;
        piece.vx = speed;
      }
    } else {
      piece.age += dt;
      piece.x += piece.vx * dt;
      piece.drop = 0.5 * FALL_G * piece.age * piece.age;
      if (piece.drop >= FALL_DEPTH) {
        state.slots[index] = null;
        onGone();
      }
    }
  }
}

function place(state: LaneState, x: number, wave: number) {
  const free = state.slots.indexOf(null);
  if (free < 0) return;
  state.slots[free] = { x, wave, falling: false, age: 0, drop: 0, vx: 0 };
}

export function step(sim: Sim, dt: number) {
  sim.clock += dt;
  const paused = isPaused(sim);
  springTo(sim.needle, sim.tenths / 10, dt, 130, 15);
  stepGate(sim.gate, paused, dt);
  sim.run += ((paused ? 0 : 1) - sim.run) * (1 - Math.exp(-dt / (paused ? 0.28 : 0.4)));
  if (sim.run < 1e-3) sim.run = 0;

  const newsSpeed = NEWS_SPEED * sim.run;
  const codeSpeed = CODE_SPEED * sim.run;

  // Pieces already on the belts move first, then new ones are dropped in behind them.
  advance(sim.news, newsSpeed, dt, () => {
    sim.sent += 1;
  });
  advance(sim.code, codeSpeed, dt, () => {});

  while (sim.news.travel >= sim.news.nextAt && sim.news.slots.includes(null)) {
    place(sim.news, NEWS_START + (sim.news.travel - sim.news.nextAt), sim.wave);
    sim.inWave += 1;
    if (sim.inWave >= WAVE_SIZE) {
      sim.inWave = 0;
      sim.wave = (sim.wave % WAVES) + 1;
      sim.news.nextAt += SPACING + WAVE_GAP;
    } else {
      sim.news.nextAt += SPACING;
    }
  }
  while (sim.code.travel >= sim.code.nextAt && sim.code.slots.includes(null)) {
    place(sim.code, CODE_START + (sim.code.travel - sim.code.nextAt), 0);
    sim.code.nextAt += CODE_SPACING;
  }
}

// Snap every spring to where it is heading (reduced motion, and the first still frame).
export function settle(sim: Sim) {
  const paused = isPaused(sim);
  sim.needle.at = sim.tenths / 10;
  sim.needle.rate = 0;
  sim.gate.at = paused ? 1 : 0;
  sim.gate.rate = 0;
  sim.run = paused ? 0 : 1;
}

export function preroll(sim: Sim, seconds: number) {
  const slice = 1 / 60;
  for (let time = 0; time < seconds; time += slice) step(sim, slice);
}

// The wave in the hopper is the one that most recently left it.
export const waveShown = (sim: Sim) => ((sim.wave + WAVES - 2 + (sim.inWave === 0 ? 0 : 1)) % WAVES) + 1;

export const bounceText = (sim: Sim) => `${(sim.tenths / 10).toFixed(1)}%`;

export function readoutOf(sim: Sim) {
  const head = isPaused(sim) ? "paused" : `wave ${waveShown(sim)} of ${WAVES}`;
  return `${head} · ${sim.sent} sent · bounce ${bounceText(sim)}`;
}

export function speechOf(sim: Sim) {
  const state = isPaused(sim) ? "Sending is paused" : `Sending wave ${waveShown(sim)} of ${WAVES}`;
  return `Bounce rate ${bounceText(sim)}. ${state}. ${sim.sent} letters sent.`;
}

// The idle tour: the knob rises past 2%, the gate drops, then it comes back down.
export const TOUR_LENGTH = 15.5;

const smooth = (t: number) => {
  const u = Math.min(1, Math.max(0, t));
  return u * u * (3 - 2 * u);
};

export function tourTenths(time: number) {
  const t = time % TOUR_LENGTH;
  const calm = 5 + 1.6 * Math.sin(t * 0.9);
  if (t < 6.5) return clampTenths(calm);
  if (t < 8.2) return clampTenths(calm + (29 - calm) * smooth((t - 6.5) / 1.7));
  if (t < 11.7) return 29;
  if (t < 13.4) return clampTenths(29 + (5 - 29) * smooth((t - 11.7) / 1.7));
  return clampTenths(calm);
}
