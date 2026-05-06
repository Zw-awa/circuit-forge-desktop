import type { GateKind } from '../types/gates';
import { ALL_GATES } from '../types/gates';

// Vite processes these SVG imports and returns URLs.
import andUrl from '../assets/gates/and.svg';
import orUrl from '../assets/gates/or.svg';
import notUrl from '../assets/gates/not.svg';
import nandUrl from '../assets/gates/nand.svg';
import norUrl from '../assets/gates/nor.svg';
import xorUrl from '../assets/gates/xor.svg';
import xnorUrl from '../assets/gates/xnor.svg';

const URLS: Record<GateKind, string> = {
  and: andUrl, or: orUrl, not: notUrl, nand: nandUrl,
  nor: norUrl, xor: xorUrl, xnor: xnorUrl,
};

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = url;
  });
}

/** A gate image and its pre-rendered white halo, for one kind. */
export interface GateAssets {
  gate: HTMLImageElement;
  /** White halo: the gate silhouette re-colored white and dilated so it
   *  reads on any desktop background. Rendered once, then drawn behind the
   *  real gate every frame (cheap). */
  halo: HTMLCanvasElement;
}

let cached: Record<GateKind, GateAssets> | null = null;

/** Natural SVG size is 100×50. We render the halo at 2× for quality, then
 *  let drawImage scale it down to the display size. */
const HALO_W = 200;
const HALO_H = 100;
/** Halo dilation in halo-canvas pixels (at 2× resolution). Adjust for thickness. */
const HALO_RADIUS = 6;

function buildHalo(img: HTMLImageElement): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = HALO_W;
  canvas.height = HALO_H;
  const ctx = canvas.getContext('2d');
  if (!ctx) return canvas;

  // Dilate: draw the gate image many times at offsets in a disk around origin,
  // all white via source-in compositing, to fatten strokes into a silhouette.
  // Pass 1: lay down all offset copies as a solid mask.
  for (let dy = -HALO_RADIUS; dy <= HALO_RADIUS; dy++) {
    for (let dx = -HALO_RADIUS; dx <= HALO_RADIUS; dx++) {
      if (dx * dx + dy * dy > HALO_RADIUS * HALO_RADIUS) continue;
      ctx.drawImage(img, dx, dy, HALO_W, HALO_H);
    }
  }
  // Pass 2: recolor everything drawn to white while keeping the shape's alpha.
  ctx.globalCompositeOperation = 'source-in';
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, HALO_W, HALO_H);
  ctx.globalCompositeOperation = 'source-over';

  return canvas;
}

/** Load all gate SVGs + pre-render their halos. Cached after first call. */
export async function loadGateImages(): Promise<Record<GateKind, GateAssets>> {
  if (cached) return cached;
  const entries = await Promise.all(
    ALL_GATES.map(async (kind) => {
      const gate = await loadImage(URLS[kind]);
      const halo = buildHalo(gate);
      return [kind, { gate, halo }] as const;
    }),
  );
  cached = Object.fromEntries(entries) as Record<GateKind, GateAssets>;
  return cached;
}

export function getGateImages(): Record<GateKind, GateAssets> | null {
  return cached;
}

