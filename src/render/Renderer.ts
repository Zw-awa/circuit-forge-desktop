import { Camera } from './Camera';
import { loadGateImages, getGateImages } from './GateImages';
import { GATE_W, GATE_H } from '../types/gates';
import type { GateKind } from '../types/gates';
import type { Component } from '../stores/circuitStore';

export interface GhostPreview { kind: GateKind; x: number; y: number; }

export class Renderer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  camera = new Camera();

  /** Called every frame to pull the current scene state. Set from outside. */
  getScene: () => { components: Component[]; ghost: GhostPreview | null } = () =>
    ({ components: [], ghost: null });

  private rafId = 0;
  private imagesReady = false;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('2D canvas context unavailable');
    this.ctx = ctx;
    void loadGateImages().then(() => { this.imagesReady = true; });
  }

  start(): void {
    const loop = () => {
      this.render();
      this.rafId = requestAnimationFrame(loop);
    };
    this.rafId = requestAnimationFrame(loop);
  }

  stop(): void {
    cancelAnimationFrame(this.rafId);
  }

  resize(cssW: number, cssH: number): void {
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.round(cssW * dpr);
    this.canvas.height = Math.round(cssH * dpr);
    this.canvas.style.width = cssW + 'px';
    this.canvas.style.height = cssH + 'px';
    this.camera.setViewport(cssW, cssH);
  }

  private render(): void {
    const { canvas, ctx, camera } = this;
    const dpr = window.devicePixelRatio || 1;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    this.drawGrid();

    if (!this.imagesReady) return;
    const { components, ghost } = this.getScene();
    for (const comp of components) {
      this.drawGate(comp.kind, comp.x, comp.y, 1);
    }
    if (ghost) {
      this.drawGate(ghost.kind, ghost.x, ghost.y, 0.5);
    }

    // Restore default transform for any callers that read from ctx afterwards.
    void camera;
  }

  private drawGrid(): void {
    const { ctx, camera } = this;
    const spacing = 1; // world cells
    const [x0w, y0w] = camera.screenToWorld(0, 0);
    const [x1w, y1w] = camera.screenToWorld(camera.screenW, camera.screenH);
    const minX = Math.floor(x0w / spacing) * spacing;
    const maxX = Math.ceil(x1w / spacing) * spacing;
    const minY = Math.floor(y0w / spacing) * spacing;
    const maxY = Math.ceil(y1w / spacing) * spacing;

    ctx.save();
    ctx.fillStyle = 'rgba(120, 120, 160, 0.35)';
    const dotR = Math.max(1, camera.zoom / 40);
    for (let wy = minY; wy <= maxY; wy += spacing) {
      for (let wx = minX; wx <= maxX; wx += spacing) {
        const [sx, sy] = camera.worldToScreen(wx, wy);
        ctx.beginPath();
        ctx.arc(sx, sy, dotR, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    ctx.restore();
  }

  private drawGate(kind: GateKind, worldX: number, worldY: number, alpha: number): void {
    const images = getGateImages();
    if (!images) return;
    const assets = images[kind];
    if (!assets) return;

    const { ctx, camera } = this;
    const [sx, sy] = camera.worldToScreen(worldX, worldY);
    const w = GATE_W * camera.zoom;
    const h = GATE_H * camera.zoom;
    const x = sx - w / 2;
    const y = sy - h / 2;

    ctx.save();
    ctx.globalAlpha = alpha;
    // Halo first (pre-rendered, no per-frame blur/shadow → no frame drops
    // that would expose the WebView2 default background as a black flash).
    ctx.drawImage(assets.halo, x, y, w, h);
    // Crisp gate on top.
    ctx.drawImage(assets.gate, x, y, w, h);
    ctx.restore();
  }
}
