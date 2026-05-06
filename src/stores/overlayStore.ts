import { create } from 'zustand';
import {
  overlaySetIgnoreCursor,
  toolbarBumpOnTop,
} from '../ipc/windowIpc';

export type PanelId =
  | 'tools'
  | 'properties'
  | 'file'
  | 'debug'
  | 'waveform'
  | 'rules'
  | 'plugins'
  | 'workshop'
  | 'keybindings'
  | 'skin';

interface OverlayState {
  /** True when overlay captures mouse (edit mode).
   *  False = overlay is click-through, desktop works normally. */
  editMode: boolean;
  /** True when toolbar position is pinned (drag handle disabled). */
  positionLocked: boolean;
  activePanel: PanelId | null;

  setEditMode: (v: boolean) => Promise<void>;
  toggleEditMode: () => Promise<void>;
  togglePositionLock: () => void;
  setActivePanel: (id: PanelId | null) => void;
}

export const useOverlayStore = create<OverlayState>((set, get) => ({
  // Default: NOT in edit mode. Overlay is click-through so taskbar, desktop,
  // and other apps work normally. User must explicitly enter edit mode to
  // draw circuits.
  editMode: false,
  positionLocked: false,
  activePanel: null,

  setEditMode: async (v: boolean) => {
    // edit mode on  = overlay captures mouse
    // edit mode off = overlay is click-through (desktop/taskbar work normally)
    // Overlay's AOT is fixed at runtime; toggling it would cause a DWM
    // redraw flash on the transparent window.
    await overlaySetIgnoreCursor(!v);
    // Re-assert toolbar on-top so it is not covered by the overlay.
    await toolbarBumpOnTop();
    set({ editMode: v });
  },

  toggleEditMode: async () => {
    await get().setEditMode(!get().editMode);
  },

  togglePositionLock: () =>
    set((s) => ({ positionLocked: !s.positionLocked })),

  setActivePanel: (id) =>
    set((s) => ({ activePanel: s.activePanel === id ? null : id })),
}));

