import type { GateKind } from './gates';
import { GATE_W, GATE_H, PINS, ALL_GATES, GATE_LABEL } from './gates';
import type { Pin } from './gates';
export type { GateKind };
export { GATE_W, GATE_H, ALL_GATES, GATE_LABEL };
export type { Pin };

export type NonGateKind =
  | 'switch' | 'led' | 'button' | 'clock' | 'random' | 'constant'
  | 'sevenSegment' | 'delayLine' | 'splitter' | 'merger';

export type ComponentKind = GateKind | NonGateKind;

export interface CompDef {
  kind: ComponentKind;
  label: string;
  category: string;
  w: number;
  h: number;
  pins: Pin[];
}

export const ALL_NON_GATE: NonGateKind[] = [
  'switch', 'led', 'button', 'clock', 'random', 'constant',
  'sevenSegment', 'delayLine', 'splitter', 'merger',
];

export const ALL_COMPONENTS: ComponentKind[] = [...ALL_GATES, ...ALL_NON_GATE];

export const CATEGORIES: Record<string, { label: string; kinds: ComponentKind[] }> = {
  gates: {
    label: 'Logic Gates',
    kinds: ALL_GATES as ComponentKind[],
  },
  io: {
    label: 'Input / Output',
    kinds: ['switch', 'led', 'button', 'sevenSegment'],
  },
  sources: {
    label: 'Signal Sources',
    kinds: ['clock', 'random', 'constant'],
  },
  utility: {
    label: 'Utility',
    kinds: ['delayLine', 'splitter', 'merger'],
  },
};

export const COMP_DEF: Record<ComponentKind, CompDef> = {
  and:  { kind: 'and',  label: 'AND',  category: 'gates', w: 2, h: 1, pins: PINS.and },
  or:   { kind: 'or',   label: 'OR',   category: 'gates', w: 2, h: 1, pins: PINS.or },
  not:  { kind: 'not',  label: 'NOT',  category: 'gates', w: 2, h: 1, pins: PINS.not },
  nand: { kind: 'nand', label: 'NAND', category: 'gates', w: 2, h: 1, pins: PINS.nand },
  nor:  { kind: 'nor',  label: 'NOR',  category: 'gates', w: 2, h: 1, pins: PINS.nor },
  xor:  { kind: 'xor',  label: 'XOR',  category: 'gates', w: 2, h: 1, pins: PINS.xor },
  xnor: { kind: 'xnor', label: 'XNOR', category: 'gates', w: 2, h: 1, pins: PINS.xnor },

  switch:      { kind: 'switch',      label: 'Switch',        category: 'io',      w: 1, h: 1, pins: [{ kind: 'out', dx: 0.5, dy: 0 }] },
  led:         { kind: 'led',         label: 'LED',           category: 'io',      w: 1, h: 1, pins: [{ kind: 'in',  dx: -0.5, dy: 0 }] },
  button:      { kind: 'button',      label: 'Button',        category: 'io',      w: 1, h: 1, pins: [{ kind: 'out', dx: 0.5, dy: 0 }] },
  sevenSegment:{ kind: 'sevenSegment',label: '7-Segment',     category: 'io',      w: 2, h: 2, pins: [
    { kind: 'in', dx: -1, dy: -0.7 }, { kind: 'in', dx: -1, dy: -0.2 },
    { kind: 'in', dx: -1, dy: 0.2 },  { kind: 'in', dx: -1, dy: 0.7 },
  ]},

  clock:    { kind: 'clock',    label: 'Clock',      category: 'sources', w: 1, h: 1, pins: [{ kind: 'out', dx: 0.5, dy: 0 }] },
  random:   { kind: 'random',   label: 'Random',     category: 'sources', w: 1, h: 1, pins: [{ kind: 'out', dx: 0.5, dy: 0 }] },
  constant: { kind: 'constant', label: 'Constant',   category: 'sources', w: 1, h: 1, pins: [{ kind: 'out', dx: 0.5, dy: 0 }] },

  delayLine: { kind: 'delayLine', label: 'Delay',       category: 'utility', w: 1.5, h: 0.6, pins: [{ kind: 'in', dx: -0.75, dy: 0 }, { kind: 'out', dx: 0.75, dy: 0 }] },
  splitter:  { kind: 'splitter',  label: 'Splitter',    category: 'utility', w: 1, h: 1.5, pins: [
    { kind: 'in', dx: -0.5, dy: 0 }, { kind: 'out', dx: 0.5, dy: -0.5 },
    { kind: 'out', dx: 0.5, dy: -0.17 }, { kind: 'out', dx: 0.5, dy: 0.17 }, { kind: 'out', dx: 0.5, dy: 0.5 },
  ]},
  merger:    { kind: 'merger',    label: 'Merger',      category: 'utility', w: 1, h: 1.5, pins: [
    { kind: 'in', dx: -0.5, dy: -0.5 }, { kind: 'in', dx: -0.5, dy: -0.17 },
    { kind: 'in', dx: -0.5, dy: 0.17 }, { kind: 'in', dx: -0.5, dy: 0.5 },
    { kind: 'out', dx: 0.5, dy: 0 },
  ]},
};

export interface PinWorldPos {
  kind: 'in' | 'out';
  x: number;
  y: number;
  pinIndex: number;
}

export const PIN_HIT_RADIUS = 0.35;

export function getPinWorldPositions(kind: ComponentKind, cx: number, cy: number): PinWorldPos[] {
  const def = COMP_DEF[kind];
  if (!def) return [];
  return def.pins.map((pin, i) => ({
    kind: pin.kind,
    x: cx + pin.dx,
    y: cy + pin.dy,
    pinIndex: i,
  }));
}
