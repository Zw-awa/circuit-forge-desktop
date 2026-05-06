import { useEffect } from 'react';
import { useOverlayStore } from './stores/overlayStore';
import { appExit } from './ipc/windowIpc';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

/** Floating Toolbar window — the only UI that is always visible.
 *  Contains two independent toggles:
 *    - positionLocked: disables the drag handle so the toolbar cannot be
 *      accidentally moved while editing
 *    - editMode:       switches the overlay between click-through (desktop
 *      works normally) and captured-mouse (you draw circuits)
 */
export default function FloatingToolbar() {
  const editMode = useOverlayStore((s) => s.editMode);
  const positionLocked = useOverlayStore((s) => s.positionLocked);
  const toggleEditMode = useOverlayStore((s) => s.toggleEditMode);
  const togglePositionLock = useOverlayStore((s) => s.togglePositionLock);
  const setEditMode = useOverlayStore((s) => s.setEditMode);

  // Global Esc: always drop out of edit mode. This is the user's emergency
  // way out if the overlay z-order ever blocks the toolbar.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && editMode) {
        void setEditMode(false);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [editMode, setEditMode]);

  // Listen for the tray's "force exit" signal so the toolbar UI stays in sync
  // after the user uses the tray escape hatch.
  useEffect(() => {
    const unlistenPromise = listen('force-exit-edit-mode', () => {
      useOverlayStore.setState({ editMode: false });
    });
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, []);

  const onGrabPointerDown = async (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    if (positionLocked) return;
    e.preventDefault();
    try {
      await getCurrentWindow().startDragging();
    } catch (err) {
      console.error('startDragging failed:', err);
    }
  };

  return (
    <div className="toolbar-root">
      <div
        className={'tb-grab' + (positionLocked ? ' disabled' : '')}
        onPointerDown={onGrabPointerDown}
        title={positionLocked ? '位置已锁定 Position locked' : '拖动 Drag'}
      >
        ⋮⋮
      </div>
      <button
        className={'tb-btn' + (positionLocked ? ' active' : '')}
        onClick={togglePositionLock}
        title={positionLocked ? '解锁位置 Unpin' : '锁定位置 Pin'}
      >
        {positionLocked ? '📌' : '📍'}
      </button>
      <button
        className={'tb-btn' + (editMode ? ' active' : '')}
        onClick={() => { void toggleEditMode(); }}
        title={editMode ? '退出编辑 Exit edit (Esc)' : '进入编辑 Enter edit mode'}
      >
        {editMode ? '✏️' : '🖱️'}
      </button>
      <span className="tb-title">
        {editMode ? 'Editing' : 'CircuitForge'}
      </span>
      <button
        className="tb-btn tb-exit"
        onClick={() => { void appExit(); }}
        title="Exit"
      >
        ✕
      </button>
    </div>
  );
}
