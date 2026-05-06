/** Camera: converts between world coordinates (cells) and screen coordinates
 *  (CSS pixels, before devicePixelRatio scaling). */
export class Camera {
  centerX = 0;
  centerY = 0;
  /** Pixels per world cell. Default 40 = a 2-cell AND gate renders as ~80 px. */
  zoom = 40;
  screenW = 0;
  screenH = 0;

  setViewport(w: number, h: number): void {
    this.screenW = w;
    this.screenH = h;
  }

  worldToScreen(wx: number, wy: number): [number, number] {
    return [
      (wx - this.centerX) * this.zoom + this.screenW / 2,
      (wy - this.centerY) * this.zoom + this.screenH / 2,
    ];
  }

  screenToWorld(sx: number, sy: number): [number, number] {
    return [
      (sx - this.screenW / 2) / this.zoom + this.centerX,
      (sy - this.screenH / 2) / this.zoom + this.centerY,
    ];
  }

  pan(dxScreen: number, dyScreen: number): void {
    this.centerX -= dxScreen / this.zoom;
    this.centerY -= dyScreen / this.zoom;
  }

  zoomAt(sx: number, sy: number, factor: number): void {
    const [wx, wy] = this.screenToWorld(sx, sy);
    this.zoom = Math.max(10, Math.min(200, this.zoom * factor));
    const [nx, ny] = this.screenToWorld(sx, sy);
    this.centerX += wx - nx;
    this.centerY += wy - ny;
  }
}
