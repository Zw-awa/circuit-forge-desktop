import { create } from 'zustand';
import type { SignalValue } from '../ipc/simulationIpc';
import { isHigh } from '../ipc/simulationIpc';

export type SimStatus = 'stopped' | 'running' | 'paused';

interface SimState {
  status: SimStatus;
  signals: Record<number, SignalValue>;
  setStatus: (s: SimStatus) => void;
  updateSignals: (changed: Record<string, SignalValue>) => void;
  clearSignals: () => void;
}

export const useSimStore = create<SimState>((set) => ({
  status: 'stopped',
  signals: {},

  setStatus: (status) => set({ status }),

  updateSignals: (changed) =>
    set((s) => {
      const next = { ...s.signals };
      for (const [key, value] of Object.entries(changed)) {
        next[Number(key)] = value;
      }
      return { signals: next };
    }),

  clearSignals: () => set({ signals: {} }),
}));

export function wireColor(signal: SignalValue | undefined, defaultColor: string): string {
  if (signal === undefined) return defaultColor;
  return isHigh(signal) ? 'rgba(80,220,120,0.85)' : 'rgba(150,150,170,0.35)';
}
