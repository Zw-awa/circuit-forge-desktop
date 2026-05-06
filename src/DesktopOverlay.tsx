import { useEffect, useRef, useState } from 'react';
import { overlayForcePassthrough, toolbarBumpOnTop } from './ipc/windowIpc';
import { useCircuitStore } from './stores/circuitStore';
import { Renderer } from './render/Renderer';
import { ALL_GATES, GATE_LABEL } from './types/gates';
import type { GateKind } from './types/gates';

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
      return { components: s.components, ghost };
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
        if (menu) { setMenu(null); return; }
        if (useCircuitStore.getState().placingKind) {
          useCircuitStore.getState().setPlacingKind(null);
          return;
        }
        void overlayForcePassthrough();
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
      if (useCircuitStore.getState().placingKind) {
        useCircuitStore.getState().setPlacingKind(null);
        setMenu(null);
        return;
      }
      setMenu({ screenX: e.clientX, screenY: e.clientY });
    };
    canvas.addEventListener('contextmenu', onContextMenu);

    const onPointerDown = (e: PointerEvent) => {
      if (e.button === 0) {
        setMenu(null);
        const [wx, wy] = renderer.camera.screenToWorld(e.clientX, e.clientY);
        const store = useCircuitStore.getState();
        if (store.placingKind) {
          store.placeAt(snap(wx), snap(wy));
          // One-shot placement: exit placing after a single drop. User must
          // re-pick from the context menu to place another.
          store.setPlacingKind(null);
        }
      }
      void toolbarBumpOnTop();
    };
    canvas.addEventListener('pointerdown', onPointerDown);

    const onPointerMove = (e: PointerEvent) => {
      const [wx, wy] = renderer.camera.screenToWorld(e.clientX, e.clientY);
      useCircuitStore.getState().setCursor(wx, wy);
    };
    canvas.addEventListener('pointermove', onPointerMove);

    return () => {
      window.removeEventListener('resize', resize);
      window.removeEventListener('keydown', onKey);
      canvas.removeEventListener('contextmenu', onContextMenu);
      canvas.removeEventListener('pointerdown', onPointerDown);
      canvas.removeEventListener('pointermove', onPointerMove);
      renderer.stop();
    };
    // The handlers close over `menu`; deps intentionally omit it so listeners
    // register once. Menu state changes use setMenu directly.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onPickGate = (kind: GateKind) => {
    useCircuitStore.getState().setPlacingKind(kind);
    setMenu(null);
  };

  const onExitEdit = () => {
    setMenu(null);
    useCircuitStore.getState().setPlacingKind(null);
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

function ContextMenu({
  x, y,
  onPickGate,
  onExitEdit,
}: {
  x: number;
  y: number;
  onPickGate: (k: GateKind) => void;
  onExitEdit: () => void;
}) {
  return (
    <div className="ctx-menu" style={{ left: x, top: y }}>
      <div className="ctx-section-title">元件 Component</div>
      {ALL_GATES.map((k) => (
        <button key={k} className="ctx-item" onClick={() => onPickGate(k)}>
          {GATE_LABEL[k]}
        </button>
      ))}
      <div className="ctx-separator" />
      <button className="ctx-item ctx-exit" onClick={onExitEdit}>
        退出编辑模式 Exit edit mode
      </button>
    </div>
  );
}
