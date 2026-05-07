import { invoke } from '@tauri-apps/api/core';

export async function saveProject(): Promise<string> {
  return invoke<string>('save_project');
}

export interface RustComponent {
  id: number;
  kind: string;
  x: number;
  y: number;
  input_pins: number[];
  output_pins: number[];
}

export interface RustWire {
  id: number;
  start: { Pin?: number; Junction?: number };
  end: { Pin?: number; Junction?: number };
  net_id: number;
}

export interface LoadProjectResult {
  components: RustComponent[];
  pins: { id: number; offset_x: number; offset_y: number }[];
  wires: RustWire[];
}

export async function loadProject(json: string): Promise<LoadProjectResult> {
  return invoke<LoadProjectResult>('load_project', { json });
}
