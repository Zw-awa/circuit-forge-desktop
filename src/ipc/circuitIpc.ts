import { invoke } from '@tauri-apps/api/core';
import type { ComponentKind } from '../types/components';

const KIND_RUST: Record<string, string> = {
  and: 'And', or: 'Or', not: 'Not',
  nand: 'Nand', nor: 'Nand', xor: 'Xor', xnor: 'Xor',
  switch: 'Switch', led: 'Led', button: 'Button',
  clock: 'Clock', random: 'Random', constant: 'Constant',
  sevenSegment: 'SevenSegment', delayLine: 'DelayLine',
  splitter: 'Splitter', merger: 'Merger',
};

export interface RustPin { id: number; offsetX: number; offsetY: number; }

export interface AddComponentResult {
  componentId: number;
  inputPins: RustPin[];
  outputPins: RustPin[];
}

export interface AddWireResult {
  wireId: number;
  netId: number;
}

export async function addComponent(
  kind: ComponentKind, x: number, y: number,
): Promise<AddComponentResult> {
  const rustKind = KIND_RUST[kind] ?? 'And';
  return invoke<AddComponentResult>('add_component', { kind: rustKind, x, y });
}

export async function removeComponent(componentId: number): Promise<void> {
  return invoke('remove_component', { componentId });
}

export async function moveComponent(
  componentId: number, x: number, y: number,
): Promise<void> {
  return invoke('move_component', { componentId, x, y });
}

export async function addWire(
  fromPinId: number, toPinId: number,
): Promise<AddWireResult> {
  return invoke<AddWireResult>('add_wire', {
    start: { Pin: fromPinId },
    end: { Pin: toPinId },
  });
}

export async function removeWire(wireId: number): Promise<void> {
  return invoke('remove_wire', { wireId });
}
