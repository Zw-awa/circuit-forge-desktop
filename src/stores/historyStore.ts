import { create } from 'zustand';
import type { Component, Wire } from './circuitStore';
import { useCircuitStore } from './circuitStore';
import type { ComponentKind } from '../types/components';

interface Undoable {
  undo(): void;
  redo(): void;
}

interface HistoryState {
  stack: Undoable[];
  pointer: number;
  push: (cmd: Undoable) => void;
  undo: () => void;
  redo: () => void;
  clear: () => void;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  stack: [],
  pointer: -1,

  push: (cmd) =>
    set((s) => ({
      stack: [...s.stack.slice(0, s.pointer + 1), cmd],
      pointer: s.pointer + 1,
    })),

  undo: () => {
    const { stack, pointer } = get();
    if (pointer < 0) return;
    stack[pointer].undo();
    set({ pointer: pointer - 1 });
  },

  redo: () => {
    const { stack, pointer } = get();
    if (pointer >= stack.length - 1) return;
    const cmd = stack[pointer + 1];
    cmd.redo();
    set({ pointer: pointer + 1 });
  },

  clear: () => set({ stack: [], pointer: -1 }),
}));

export function pushPlace(kind: ComponentKind, x: number, y: number, compId: number) {
  useHistoryStore.getState().push({
    undo: () => useCircuitStore.getState().removeById(compId),
    redo: () => {
      useCircuitStore.getState().setPlacingKind(kind);
      useCircuitStore.getState().placeAt(x, y);
    },
  });
}

export function pushRemove(comp: Component, wires: Wire[]) {
  useHistoryStore.getState().push({
    undo: () => {
      const s = useCircuitStore.getState();
      useCircuitStore.setState({
        components: [...s.components, comp].sort((a, b) => a.id - b.id),
        wires: [...s.wires, ...wires],
      });
    },
    redo: () => useCircuitStore.getState().removeById(comp.id),
  });
}

export function pushMove(compId: number, oldX: number, oldY: number, newX: number, newY: number) {
  useHistoryStore.getState().push({
    undo: () => useCircuitStore.getState().moveComponent(compId, oldX, oldY),
    redo: () => useCircuitStore.getState().moveComponent(compId, newX, newY),
  });
}

export function pushWire(wire: Wire) {
  useHistoryStore.getState().push({
    undo: () => useCircuitStore.setState((s) => ({ wires: s.wires.filter((w) => w.id !== wire.id) })),
    redo: () => useCircuitStore.setState((s) => ({ wires: [...s.wires, wire] })),
  });
}

