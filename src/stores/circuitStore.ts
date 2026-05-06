import { create } from 'zustand';
import type { GateKind } from '../types/gates';

export interface Component {
  id: number;
  kind: GateKind;
  /** World-space position of the component center (in cells). */
  x: number;
  y: number;
}

interface CircuitState {
  components: Component[];
  nextId: number;

  /** Kind currently selected for placement. null = no active placement. */
  placingKind: GateKind | null;
  /** Live cursor position in world coordinates (for ghost preview). */
  cursorX: number;
  cursorY: number;

  setPlacingKind: (kind: GateKind | null) => void;
  setCursor: (wx: number, wy: number) => void;
  placeAt: (wx: number, wy: number) => void;
  removeAt: (wx: number, wy: number) => void;
  clearAll: () => void;
}

export const useCircuitStore = create<CircuitState>((set, get) => ({
  components: [],
  nextId: 1,
  placingKind: null,
  cursorX: 0,
  cursorY: 0,

  setPlacingKind: (kind) => set({ placingKind: kind }),

  setCursor: (wx, wy) => set({ cursorX: wx, cursorY: wy }),

  placeAt: (wx, wy) => {
    const kind = get().placingKind;
    if (!kind) return;
    set((s) => ({
      components: [...s.components, { id: s.nextId, kind, x: wx, y: wy }],
      nextId: s.nextId + 1,
    }));
  },

  removeAt: (wx, wy) => {
    // Delete topmost component whose bounding box contains (wx, wy).
    // Bounding box: 2 wide, 1 tall, centered on (x, y).
    set((s) => {
      for (let i = s.components.length - 1; i >= 0; i--) {
        const c = s.components[i];
        if (Math.abs(wx - c.x) <= 1 && Math.abs(wy - c.y) <= 0.5) {
          return { components: s.components.filter((_, j) => j !== i) };
        }
      }
      return {};
    });
  },

  clearAll: () => set({ components: [], nextId: 1 }),
}));
