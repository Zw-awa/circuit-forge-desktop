import { create } from 'zustand';
import type { ComponentKind, PinWorldPos } from '../types/components';
import { getPinWorldPositions } from '../types/components';
import { addComponent, removeComponent, moveComponent as rustMove, addWire, removeWire } from '../ipc/circuitIpc';
import type { RustPin } from '../ipc/circuitIpc';

export interface Component {
  id: number;
  kind: ComponentKind;
  x: number;
  y: number;
  rustId?: number;
  rustPins?: { in: RustPin[]; out: RustPin[] };
}

export interface Wire {
  id: number;
  fromComponentId: number;
  fromPinIndex: number;
  toComponentId: number;
  toPinIndex: number;
  rustId?: number;
  netId?: number;
}

export interface PlacingWire {
  fromComponentId: number;
  fromPinIndex: number;
  fromX: number;
  fromY: number;
}

interface CircuitState {
  components: Component[];
  nextId: number;

  /** Kind currently selected for placement. null = no active placement. */
  placingKind: ComponentKind | null;
  cursorX: number;
  cursorY: number;
  selectedId: number | null;
  wires: Wire[];
  nextWireId: number;
  placingWire: PlacingWire | null;

  setPlacingKind: (kind: ComponentKind | null) => void;
  setCursor: (wx: number, wy: number) => void;
  placeAt: (wx: number, wy: number) => void;
  removeAt: (wx: number, wy: number) => void;
  clearAll: () => void;
  selectAt: (wx: number, wy: number) => void;
  clearSelection: () => void;
  removeById: (id: number) => void;
  moveComponent: (id: number, x: number, y: number) => void;
  startWire: (componentId: number, pinIndex: number) => void;
  completeWire: (componentId: number, pinIndex: number) => void;
  cancelWire: () => void;
  getPinPositions: (componentId: number) => PinWorldPos[] | null;
}

export const useCircuitStore = create<CircuitState>((set, get) => ({
  components: [],
  nextId: 1,
  placingKind: null,
  cursorX: 0,
  cursorY: 0,
  selectedId: null,
  wires: [],
  nextWireId: 1,
  placingWire: null,

  setPlacingKind: (kind) => set({ placingKind: kind }),

  setCursor: (wx, wy) => set({ cursorX: wx, cursorY: wy }),

  placeAt: (wx, wy) => {
    const kind = get().placingKind;
    if (!kind) return;
    const newId = get().nextId;
    set((s) => ({
      components: [...s.components, { id: s.nextId, kind, x: wx, y: wy }],
      nextId: s.nextId + 1,
    }));
    addComponent(kind, wx, wy)
      .then((res) => set((s) => ({
        components: s.components.map((c) =>
          c.id === newId
            ? { ...c, rustId: res.componentId, rustPins: { in: res.inputPins, out: res.outputPins } }
            : c,
        ),
      })))
      .catch((e) => console.error('add_component failed:', e));
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

  clearAll: () => set({ components: [], nextId: 1, wires: [], nextWireId: 1 }),

  selectAt: (wx, wy) => {
    const id = hitTest(get().components, wx, wy);
    set({ selectedId: id });
  },

  clearSelection: () => set({ selectedId: null }),

  removeById: (id) => {
    const comp = get().components.find((c) => c.id === id);
    const rustId = comp?.rustId;
    const wireIds = get().wires
      .filter((w) => (w.fromComponentId === id || w.toComponentId === id) && w.rustId != null)
      .map((w) => w.rustId!);
    set((s) => ({
      components: s.components.filter((c) => c.id !== id),
      wires: s.wires.filter((w) => w.fromComponentId !== id && w.toComponentId !== id),
      selectedId: s.selectedId === id ? null : s.selectedId,
    }));
    if (rustId != null) {
      removeComponent(rustId).catch((e) => console.error('remove_component failed:', e));
      for (const wid of wireIds) {
        removeWire(wid).catch((e) => console.error('remove_wire failed:', e));
      }
    }
  },

  moveComponent: (id, x, y) => {
    set((s) => ({
      components: s.components.map((c) => (c.id === id ? { ...c, x, y } : c)),
    }));
    const comp = get().components.find((c) => c.id === id);
    if (comp?.rustId != null) {
      rustMove(comp.rustId, x, y).catch((e) => console.error('move_component failed:', e));
    }
  },

  startWire: (componentId, pinIndex) => {
    const comp = get().components.find((c) => c.id === componentId);
    if (!comp) return;
    const pins = getPinWorldPositions(comp.kind, comp.x, comp.y);
    const pin = pins[pinIndex];
    if (!pin || pin.kind !== 'out') return;
    set({
      placingWire: {
        fromComponentId: componentId,
        fromPinIndex: pinIndex,
        fromX: pin.x,
        fromY: pin.y,
      },
    });
  },

  completeWire: (componentId, pinIndex) => {
    const pw = get().placingWire;
    if (!pw) return;
    if (componentId === pw.fromComponentId) return;
    const toComp = get().components.find((c) => c.id === componentId);
    if (!toComp) return;
    const pins = getPinWorldPositions(toComp.kind, toComp.x, toComp.y);
    const pin = pins[pinIndex];
    if (!pin || pin.kind !== 'in') return;
    const newId = get().nextWireId;
    set((s) => ({
      wires: [...s.wires, {
        id: s.nextWireId,
        fromComponentId: pw.fromComponentId,
        fromPinIndex: pw.fromPinIndex,
        toComponentId: componentId,
        toPinIndex: pinIndex,
      }],
      nextWireId: s.nextWireId + 1,
      placingWire: null,
    }));
    const fromComp = get().components.find((c) => c.id === pw.fromComponentId);
    const fromRustPins = fromComp?.rustPins;
    const toRustPins = toComp.rustPins;
    if (fromRustPins && toRustPins) {
      const fromPinId = fromRustPins.out[pw.fromPinIndex]?.id;
      const toPinId = toRustPins.in[pinIndex]?.id;
      if (fromPinId != null && toPinId != null) {
        addWire(fromPinId, toPinId)
          .then((res) => set((s) => ({
            wires: s.wires.map((w) =>
              w.id === newId ? { ...w, rustId: res.wireId, netId: res.netId } : w,
            ),
          })))
          .catch((e) => console.error('add_wire failed:', e));
      }
    }
  },

  cancelWire: () => set({ placingWire: null }),

  getPinPositions: (componentId) => {
    const comp = get().components.find((c) => c.id === componentId);
    if (!comp) return null;
    return getPinWorldPositions(comp.kind, comp.x, comp.y);
  },
}));

function hitTest(components: Component[], wx: number, wy: number): number | null {
  for (let i = components.length - 1; i >= 0; i--) {
    const c = components[i];
    if (Math.abs(wx - c.x) <= 1 && Math.abs(wy - c.y) <= 0.5) {
      return c.id;
    }
  }
  return null;
}
