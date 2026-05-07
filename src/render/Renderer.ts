import { Camera } from './Camera';
import { loadGateImages, getGateImages } from './GateImages';
import { ALL_GATES } from '../types/gates';
import type { GateKind } from '../types/gates';
import type { ComponentKind } from '../types/components';
import { COMP_DEF } from '../types/components';
import type { Component } from '../stores/circuitStore';

const GATE_SET = new Set<string>(ALL_GATES);

export interface GhostPreview { kind: ComponentKind; x: number; y: number; }
export interface WireSegment { fromX: number; fromY: number; toX: number; toY: number; color: string; }
export interface WirePreview { fromX: number; fromY: number; toX: number; toY: number; }

export class Renderer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  camera = new Camera();

  getScene: () => {
    components: Component[];
    ghost: GhostPreview | null;
    selectedId: number | null;
    wireSegments: WireSegment[];
    wirePreview: WirePreview | null;
  } = () =>
    ({ components: [], ghost: null, selectedId: null, wireSegments: [], wirePreview: null });

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
    const loop = () => { this.render(); this.rafId = requestAnimationFrame(loop); };
    this.rafId = requestAnimationFrame(loop);
  }

  stop(): void { cancelAnimationFrame(this.rafId); }

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
    const { components, ghost, selectedId, wireSegments, wirePreview } = this.getScene();

    for (const seg of wireSegments) {
      this.drawWireLine(seg.fromX, seg.fromY, seg.toX, seg.toY, seg.color);
    }
    if (wirePreview) {
      this.drawWireLine(wirePreview.fromX, wirePreview.fromY, wirePreview.toX, wirePreview.toY, 'rgba(120,180,220,0.35)');
    }

    for (const comp of components) {
      this.drawComponent(comp.kind, comp.x, comp.y, 1, comp.id === selectedId);
    }
    if (ghost) {
      this.drawComponent(ghost.kind, ghost.x, ghost.y, 0.5, false);
    }
    void camera;
  }

  private drawGrid(): void {
    const { ctx, camera } = this;
    const spacing = 1;
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

  private drawWireLine(wx0: number, wy0: number, wx1: number, wy1: number, color: string): void {
    const { ctx, camera } = this;
    const [sx0, sy0] = camera.worldToScreen(wx0, wy0);
    const [sx1, sy1] = camera.worldToScreen(wx1, wy1);
    ctx.save();
    ctx.strokeStyle = color;
    ctx.lineWidth = Math.max(1.5, camera.zoom / 30);
    ctx.beginPath();
    ctx.moveTo(sx0, sy0);
    ctx.lineTo(sx1, sy1);
    ctx.stroke();
    ctx.restore();
  }

  private drawComponent(
    kind: ComponentKind, worldX: number, worldY: number, alpha: number, selected: boolean,
  ): void {
    if (GATE_SET.has(kind)) {
      this.drawGateImage(kind, worldX, worldY, alpha, selected);
    } else {
      this.drawNonGate(kind, worldX, worldY, alpha, selected);
    }
  }

  private drawGateImage(
    kind: ComponentKind, worldX: number, worldY: number, alpha: number, selected: boolean,
  ): void {
    const images = getGateImages();
    if (!images) return;
    const def = COMP_DEF[kind];
    const gk = kind as GateKind;
    const assets = images[gk];
    if (!assets || !def) return;

    const { ctx, camera } = this;
    const [sx, sy] = camera.worldToScreen(worldX, worldY);
    const w = def.w * camera.zoom;
    const h = def.h * camera.zoom;
    const x = sx - w / 2;
    const y = sy - h / 2;

    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.drawImage(assets.halo, x, y, w, h);
    ctx.drawImage(assets.gate, x, y, w, h);
    if (selected) {
      const inset = Math.max(1, camera.zoom / 20);
      ctx.globalAlpha = 1;
      ctx.strokeStyle = 'rgba(100, 180, 255, 0.9)';
      ctx.lineWidth = Math.max(1.5, camera.zoom / 25);
      ctx.strokeRect(x + inset, y + inset, w - inset * 2, h - inset * 2);
    }
    ctx.restore();
  }

  private drawNonGate(
    kind: ComponentKind, worldX: number, worldY: number, alpha: number, selected: boolean,
  ): void {
    const def = COMP_DEF[kind];
    if (!def) return;
    const { ctx, camera } = this;
    const [sx, sy] = camera.worldToScreen(worldX, worldY);
    const z = camera.zoom;
    const w = def.w * z;
    const h = def.h * z;
    const x = sx - w / 2;
    const y = sy - h / 2;

    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.fillStyle = 'rgba(60, 60, 80, 0.85)';
    ctx.strokeStyle = 'rgba(160, 180, 200, 0.8)';
    ctx.lineWidth = Math.max(1, z / 40);
    const r = Math.min(w, h) / 2;

    switch (kind) {
      case 'switch': {
        ctx.beginPath(); ctx.arc(sx, sy, r * 0.8, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(sx, sy - r * 0.4); ctx.lineTo(sx + r * 0.6, sy + r * 0.3); ctx.stroke();
        break;
      }
      case 'button': {
        ctx.beginPath(); ctx.arc(sx, sy, r * 0.7, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
        ctx.fillStyle = 'rgba(220, 220, 240, 0.9)';
        ctx.beginPath(); ctx.arc(sx, sy, r * 0.3, 0, Math.PI * 2); ctx.fill();
        break;
      }
      case 'led': {
        ctx.beginPath(); ctx.arc(sx, sy, r * 0.7, 0, Math.PI * 2); ctx.fill();
        ctx.fillStyle = 'rgba(220, 80, 80, 0.7)';
        ctx.beginPath(); ctx.arc(sx, sy, r * 0.5, 0, Math.PI * 2); ctx.fill();
        ctx.strokeStyle = 'rgba(160, 180, 200, 0.6)';
        ctx.beginPath(); ctx.arc(sx, sy, r * 0.7, 0, Math.PI * 2); ctx.stroke();
        break;
      }
      case 'clock': {
        ctx.fillRect(x, y, w, h); ctx.strokeRect(x, y, w, h);
        ctx.strokeStyle = 'rgba(200, 220, 255, 0.7)';
        ctx.beginPath(); ctx.moveTo(sx, sy); ctx.lineTo(sx, sy - h * 0.35); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(sx, sy); ctx.lineTo(sx + w * 0.25, sy); ctx.stroke();
        break;
      }
      case 'random': {
        ctx.fillRect(x, y, w, h); ctx.strokeRect(x, y, w, h);
        ctx.fillStyle = 'rgba(200, 200, 220, 0.8)';
        ctx.font = `${z * 0.6}px sans-serif`; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
        ctx.fillText('?', sx, sy);
        break;
      }
      case 'constant': {
        ctx.fillRect(x, y, w, h); ctx.strokeRect(x, y, w, h);
        ctx.fillStyle = 'rgba(200, 220, 200, 0.8)';
        ctx.font = `${z * 0.5}px sans-serif`; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
        ctx.fillText('1', sx, sy);
        break;
      }
      case 'sevenSegment': {
        ctx.fillRect(x, y, w, h); ctx.strokeRect(x, y, w, h);
        ctx.fillStyle = 'rgba(220, 100, 100, 0.6)';
        ctx.fillRect(x + w * 0.15, y + h * 0.1, w * 0.7, h * 0.1);
        ctx.fillRect(x + w * 0.8, y + h * 0.12, w * 0.1, h * 0.33);
        ctx.fillRect(x + w * 0.8, y + h * 0.55, w * 0.1, h * 0.33);
        ctx.fillRect(x + w * 0.15, y + h * 0.8, w * 0.7, h * 0.1);
        ctx.fillRect(x + w * 0.05, y + h * 0.55, w * 0.1, h * 0.33);
        ctx.fillRect(x + w * 0.05, y + h * 0.12, w * 0.1, h * 0.33);
        ctx.fillRect(x + w * 0.15, y + h * 0.45, w * 0.7, h * 0.1);
        break;
      }
      case 'delayLine': {
        ctx.fillRect(x, y + h * 0.2, w, h * 0.6); ctx.strokeRect(x, y + h * 0.2, w, h * 0.6);
        ctx.strokeStyle = 'rgba(180, 200, 255, 0.8)';
        ctx.beginPath(); ctx.moveTo(x + w * 0.2, y + h * 0.4); ctx.lineTo(x + w * 0.6, y + h * 0.4);
        ctx.lineTo(x + w * 0.5, y + h * 0.2); ctx.moveTo(x + w * 0.6, y + h * 0.4);
        ctx.lineTo(x + w * 0.5, y + h * 0.6); ctx.stroke();
        break;
      }
      case 'splitter': {
        ctx.fillRect(x, y, w, h); ctx.strokeRect(x, y, w, h);
        ctx.strokeStyle = 'rgba(180, 220, 180, 0.7)';
        const by = y + h * 0.3;
        ctx.beginPath(); ctx.moveTo(x + w * 0.1, sy); ctx.lineTo(x + w * 0.6, by - h * 0.15);
        ctx.moveTo(x + w * 0.1, sy); ctx.lineTo(x + w * 0.6, by + h * 0.15);
        ctx.moveTo(x + w * 0.1, sy); ctx.lineTo(x + w * 0.6, by + h * 0.45);
        ctx.stroke();
        break;
      }
      case 'merger': {
        ctx.fillRect(x, y, w, h); ctx.strokeRect(x, y, w, h);
        ctx.strokeStyle = 'rgba(220, 180, 180, 0.7)';
        const my = y + h * 0.3;
        ctx.beginPath(); ctx.moveTo(x + w * 0.9, sy); ctx.lineTo(x + w * 0.4, my - h * 0.15);
        ctx.moveTo(x + w * 0.9, sy); ctx.lineTo(x + w * 0.4, my + h * 0.15);
        ctx.moveTo(x + w * 0.9, sy); ctx.lineTo(x + w * 0.4, my + h * 0.45);
        ctx.stroke();
        break;
      }
    }

    if (selected) {
      const inset = Math.max(1, z / 20);
      ctx.globalAlpha = 1;
      ctx.strokeStyle = 'rgba(100, 180, 255, 0.9)';
      ctx.lineWidth = Math.max(1.5, z / 25);
      ctx.strokeRect(x + inset, y + inset, w - inset * 2, h - inset * 2);
    }
    ctx.restore();
  }
}
