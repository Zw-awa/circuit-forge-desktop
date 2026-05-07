import { invoke } from '@tauri-apps/api/core';

export type SignalValue = 'High' | 'Low' | { Bus: number } | { Integer: number } | { Float: number };

export interface SignalMap {
  [netId: string]: SignalValue;
}

export interface SimTickPayload {
  tick: number;
  changed: SignalMap;
}

export async function simStart(): Promise<void> {
  return invoke('sim_start');
}

export async function simPause(): Promise<void> {
  return invoke('sim_pause');
}

export async function simStep(): Promise<SignalMap> {
  return invoke<SignalMap>('sim_step');
}

export async function simReset(): Promise<void> {
  return invoke('sim_reset');
}

export async function getSignals(): Promise<SignalMap> {
  return invoke<SignalMap>('get_signals');
}

export function isHigh(s: SignalValue): boolean {
  if (s === 'High') return true;
  if (s === 'Low') return false;
  if (typeof s === 'object') {
    if ('Bus' in s) return s.Bus !== 0;
    if ('Integer' in s) return s.Integer !== 0;
    if ('Float' in s) return s.Float !== 0;
  }
  return false;
}
