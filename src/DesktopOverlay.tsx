import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { overlayForcePassthrough, toolbarBumpOnTop } from './ipc/windowIpc';
import { useCircuitStore } from './stores/circuitStore';
import type { Component } from './stores/circuitStore';
import { useSimStore, wireColor } from './stores/simStore';
import { Renderer } from './render/Renderer';
import { ALL_GATES, GATE_LABEL } from './types/gates';
import type { ComponentKind } from './types/components';
import { COMP_DEF, getPinWorldPositions, PIN_HIT_RADIUS } from './types/components';

interface ContextMenuState {
  screenX: number;
  screenY: number;
}

/** Desktop Overlay window — fullscreen transparent canvas.
 *  Captures mouse only when the toolbar tells us to enter edit mode.
 *  Right-click opens a contextual menu similar to the Windows desktop. */
export default function DesktopOverlay() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rendererRef = useRef<Renderer | null>(null);
  const [menu, setMenu] = useState<ContextMenuState | null>(null);

  // Panning state via refs (not state) so the rAF-driven effect handlers
  // always read the latest values without triggering re-renders.
  const isPanningRef = useRef(false);
  const lastPanXRef = useRef(0);
  const lastPanYRef = useRef(0);

  const isDraggingRef = useRef(false);
  const dragCompIdRef = useRef<number | null>(null);
  const dragOffXRef = useRef(0);
  const dragOffYRef = useRef(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const renderer = new Renderer(canvas);
    rendererRef.current = renderer;
    renderer.getScene = () => {
      const s = useCircuitStore.getState();
      const ghost = s.placingKind
        ? { kind: s.placingKind, x: snap(s.cursorX), y: snap(s.cursorY) }
        : null;

      const wireSegments = s.wires.map((w) => {
        const from = s.components.find((c) => c.id === w.fromComponentId);
        const to = s.components.find((c) => c.id === w.toComponentId);
        if (!from || !to) return null;
        const fp = getPinWorldPositions(from.kind, from.x, from.y)[w.fromPinIndex];
        const tp = getPinWorldPositions(to.kind, to.x, to.y)[w.toPinIndex];
        if (!fp || !tp) return null;
        const signals = useSimStore.getState().signals;
        const color = w.netId != null
          ? wireColor(signals[w.netId], 'rgba(120,180,220,0.6)')
          : 'rgba(120,180,220,0.6)';
        return { fromX: fp.x, fromY: fp.y, toX: tp.x, toY: tp.y, color };
      }).filter(Boolean) as { fromX: number; fromY: number; toX: number; toY: number; color: string }[];

      const wirePreview = s.placingWire
        ? { fromX: s.placingWire.fromX, fromY: s.placingWire.fromY, toX: s.cursorX, toY: s.cursorY }
        : null;

      return { components: s.components, ghost, selectedId: s.selectedId, wireSegments, wirePreview };
    };

    const resize = () => renderer.resize(window.innerWidth, window.innerHeight);
    resize();
    window.addEventListener('resize', resize);

    renderer.start();

    // Esc:
    //   1. close the context menu if open
    //   2. otherwise cancel the current placement if any
    //   3. otherwise exit edit mode entirely
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (isDraggingRef.current) {
          isDraggingRef.current = false;
          dragCompIdRef.current = null;
          return;
        }
        if (useCircuitStore.getState().placingWire) {
          useCircuitStore.getState().cancelWire();
          return;
        }
        if (menu) { setMenu(null); return; }
        if (useCircuitStore.getState().placingKind) {
          useCircuitStore.getState().setPlacingKind(null);
          return;
        }
        void overlayForcePassthrough();
        return;
      }
      if (e.key === 'Delete' || e.key === 'Backspace') {
        const id = useCircuitStore.getState().selectedId;
        if (id !== null) {
          useCircuitStore.getState().removeById(id);
        }
      }
    };
    window.addEventListener('keydown', onKey);

    // Right-click semantics:
    //   - if a placement is active, right-click cancels the placement
    //     (no menu opens). This matches Blender / CAD conventions and gives
    //     the user an ergonomic one-handed way to stop placing.
    //   - if nothing is being placed, right-click opens the context menu.
    const onContextMenu = (e: MouseEvent) => {
      e.preventDefault();
      if (useCircuitStore.getState().placingWire) {
        useCircuitStore.getState().cancelWire();
        return;
      }
      if (useCircuitStore.getState().placingKind) {
        useCircuitStore.getState().setPlacingKind(null);
        setMenu(null);
        return;
      }
      setMenu({ screenX: e.clientX, screenY: e.clientY });
    };
    canvas.addEventListener('contextmenu', onContextMenu);

    const onPointerDown = (e: PointerEvent) => {
      if (e.button === 1) {
        e.preventDefault();
        isPanningRef.current = true;
        lastPanXRef.current = e.clientX;
        lastPanYRef.current = e.clientY;
        return;
      }
      if (e.button === 0) {
        setMenu(null);
        const [wx, wy] = renderer.camera.screenToWorld(e.clientX, e.clientY);
        const store = useCircuitStore.getState();
        if (store.placingKind) {
          store.placeAt(snap(wx), snap(wy));
          store.setPlacingKind(null);
        } else if (store.placingWire) {
          const pinHit = hitTestPin(store.components, wx, wy);
          if (pinHit && pinHit.kind === 'in') {
            store.completeWire(pinHit.componentId, pinHit.pinIndex);
          } else {
            store.cancelWire();
          }
        } else {
          const pinHit = hitTestPin(store.components, wx, wy);
          if (pinHit && pinHit.kind === 'out') {
            store.startWire(pinHit.componentId, pinHit.pinIndex);
          } else {
            store.selectAt(wx, wy);
            const id = useCircuitStore.getState().selectedId;
            if (id !== null) {
              const comp = useCircuitStore.getState().components.find((c) => c.id === id);
              if (comp) {
                isDraggingRef.current = true;
                dragCompIdRef.current = id;
                dragOffXRef.current = comp.x - wx;
                dragOffYRef.current = comp.y - wy;
              }
            }
          }
        }
      }
      void toolbarBumpOnTop();
    };
    canvas.addEventListener('pointerdown', onPointerDown);

    const onPointerMove = (e: PointerEvent) => {
      if (isPanningRef.current) {
        const dx = e.clientX - lastPanXRef.current;
        const dy = e.clientY - lastPanYRef.current;
        lastPanXRef.current = e.clientX;
        lastPanYRef.current = e.clientY;
        renderer.camera.pan(dx, dy);
        return;
      }
      const [wx, wy] = renderer.camera.screenToWorld(e.clientX, e.clientY);
      if (isDraggingRef.current && dragCompIdRef.current !== null) {
        useCircuitStore.getState().moveComponent(
          dragCompIdRef.current,
          snap(wx + dragOffXRef.current),
          snap(wy + dragOffYRef.current),
        );
        return;
      }
      useCircuitStore.getState().setCursor(wx, wy);
    };
    canvas.addEventListener('pointermove', onPointerMove);

    const onPointerUp = (e: PointerEvent) => {
      if (e.button === 1) {
        isPanningRef.current = false;
      }
      if (e.button === 0) {
        isDraggingRef.current = false;
        dragCompIdRef.current = null;
      }
    };
    window.addEventListener('pointerup', onPointerUp);

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const factor = e.deltaY < 0 ? 1.1 : 0.9;
      renderer.camera.zoomAt(e.clientX, e.clientY, factor);
    };
    canvas.addEventListener('wheel', onWheel, { passive: false });

    return () => {
      window.removeEventListener('resize', resize);
      window.removeEventListener('keydown', onKey);
      canvas.removeEventListener('contextmenu', onContextMenu);
      canvas.removeEventListener('pointerdown', onPointerDown);
      canvas.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      canvas.removeEventListener('wheel', onWheel);
      renderer.stop();
    };
    // The handlers close over `menu`; deps intentionally omit it so listeners
    // register once. Menu state changes use setMenu directly.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const p = listen('overlay-delete-component', () => {
      const id = useCircuitStore.getState().selectedId;
      if (id !== null) { useCircuitStore.getState().removeById(id); }
    });
    return () => { p.then((u) => u()); };
  }, []);

  const onPickGate = (kind: ComponentKind) => {
    useCircuitStore.getState().setPlacingKind(kind);
    setMenu(null);
  };

  const onExitEdit = () => {
    setMenu(null);
    useCircuitStore.getState().setPlacingKind(null);
    useCircuitStore.getState().cancelWire();
    void overlayForcePassthrough();
  };

  return (
    <div className="overlay-root">
      <canvas ref={canvasRef} className="overlay-canvas" />
      {menu && (
        <ContextMenu
          x={menu.screenX}
          y={menu.screenY}
          onPickGate={onPickGate}
          onExitEdit={onExitEdit}
        />
      )}
    </div>
  );
}

/** Snap a world coordinate to the nearest half-cell so gates align to the
 *  dot grid. Gates are 2 wide × 1 tall; their centers land on integer y and
 *  integer x. */
function snap(v: number): number {
  return Math.round(v);
}

function hitTestPin(
  components: Component[],
  wx: number,
  wy: number,
): { componentId: number; pinIndex: number; kind: 'in' | 'out' } | null {
  for (let i = components.length - 1; i >= 0; i--) {
    const c = components[i];
    const pins = getPinWorldPositions(c.kind, c.x, c.y);
    for (const pin of pins) {
      if (Math.hypot(wx - pin.x, wy - pin.y) <= PIN_HIT_RADIUS) {
        return { componentId: c.id, pinIndex: pin.pinIndex, kind: pin.kind };
      }
    }
  }
  return null;
}

function ContextMenu({
  x, y,
  onPickGate,
  onExitEdit,
}: {
  x: number;
  y: number;
  onPickGate: (k: ComponentKind) => void;
  onExitEdit: () => void;
}) {
  return (
    <div className="ctx-menu" style={{ left: x, top: y }}>
      <div className="ctx-section-title">Logic Gates</div>
      {ALL_GATES.map((k) => (
        <button key={k} className="ctx-item" onClick={() => onPickGate(k)}>
          {GATE_LABEL[k]}
        </button>
      ))}
      <div className="ctx-separator" />
      <div className="ctx-section-title">I/O & Sources</div>
      {(['switch', 'led', 'button', 'clock', 'constant'] as ComponentKind[]).map((k) => (
        <button key={k} className="ctx-item" onClick={() => onPickGate(k)}>
          {COMP_DEF[k].label}
        </button>
      ))}
      <div className="ctx-separator" />
      <button className="ctx-item ctx-exit" onClick={onExitEdit}>
        退出编辑模式 Exit edit mode
      </button>
    </div>
  );
}
