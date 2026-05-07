/** Built-in logic gate kinds. Matches the Rust `ComponentKind` enum names
 *  (lowercase here to match SVG filenames). */
export type GateKind = 'and' | 'or' | 'not' | 'nand' | 'nor' | 'xor' | 'xnor';

export const ALL_GATES: GateKind[] = ['and', 'or', 'not', 'nand', 'nor', 'xor', 'xnor'];

export const GATE_LABEL: Record<GateKind, string> = {
  and: 'AND',
  or: 'OR',
  not: 'NOT',
  nand: 'NAND',
  nor: 'NOR',
  xor: 'XOR',
  xnor: 'XNOR',
};

/** Component dimensions in world cells. Derived from the SVG viewport
 *  (100×50 px → 2×1 cells at the natural scale of 50 px per cell). */
export const GATE_W = 2;
export const GATE_H = 1;

/** Pin offsets (in cells, relative to component center at 0,0).
 *  Matches where the SVG pin stubs start/end:
 *    AND/OR/NAND/NOR/XOR/XNOR: two inputs on the left, one output on the right
 *    NOT: single input left, single output right */
export interface Pin { kind: 'in' | 'out'; dx: number; dy: number; }

export const PINS: Record<GateKind, Pin[]> = {
  and:  [{ kind: 'in', dx: -1, dy: -0.3 }, { kind: 'in', dx: -1, dy: 0.3 }, { kind: 'out', dx: 1, dy: 0 }],
  or:   [{ kind: 'in', dx: -1, dy: -0.3 }, { kind: 'in', dx: -1, dy: 0.3 }, { kind: 'out', dx: 1, dy: 0 }],
  nand: [{ kind: 'in', dx: -1, dy: -0.3 }, { kind: 'in', dx: -1, dy: 0.3 }, { kind: 'out', dx: 1, dy: 0 }],
  nor:  [{ kind: 'in', dx: -1, dy: -0.3 }, { kind: 'in', dx: -1, dy: 0.3 }, { kind: 'out', dx: 1, dy: 0 }],
  xor:  [{ kind: 'in', dx: -1, dy: -0.3 }, { kind: 'in', dx: -1, dy: 0.3 }, { kind: 'out', dx: 1, dy: 0 }],
  xnor: [{ kind: 'in', dx: -1, dy: -0.3 }, { kind: 'in', dx: -1, dy: 0.3 }, { kind: 'out', dx: 1, dy: 0 }],
  not:  [{ kind: 'in', dx: -1, dy: 0 }, { kind: 'out', dx: 1, dy: 0 }],
};

export interface PinWorldPos {
  kind: 'in' | 'out';
  x: number;
  y: number;
  pinIndex: number;
}

export const PIN_HIT_RADIUS = 0.35;

export function getPinWorldPositions(kind: GateKind, cx: number, cy: number): PinWorldPos[] {
  return (PINS[kind] || []).map((pin, i) => ({
    kind: pin.kind,
    x: cx + pin.dx,
    y: cy + pin.dy,
    pinIndex: i,
  }));
}
