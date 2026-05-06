import { invoke } from '@tauri-apps/api/core';

/** Toggle the overlay window's click-through state.
 *  true  = clicks pass through to desktop (locked mode)
 *  false = overlay captures mouse events (unlocked mode) */
export async function overlaySetIgnoreCursor(ignore: boolean): Promise<void> {
  return invoke('overlay_set_ignore_cursor', { ignore });
}

/** Toggle the overlay's always-on-top flag and bump the toolbar so it stays
 *  above the overlay regardless. Called on edit-mode transitions. */
export async function overlaySetAlwaysOnTop(onTop: boolean): Promise<void> {
  return invoke('overlay_set_always_on_top', { onTop });
}

export async function toolbarSetSize(width: number, height: number): Promise<void> {
  return invoke('toolbar_set_size', { width, height });
}

export async function toolbarSetPosition(x: number, y: number): Promise<void> {
  return invoke('toolbar_set_position', { x, y });
}

export async function appExit(): Promise<void> {
  return invoke('app_exit');
}

/** Re-assert toolbar's always-on-top. Call right after toggling edit mode so
 *  a freshly-captured overlay cannot cover the toolbar. */
export async function toolbarBumpOnTop(): Promise<void> {
  return invoke('toolbar_bump_on_top');
}

/** Emergency escape: forces overlay back to click-through, bumps toolbar,
 *  and emits an event so any UI state syncs. */
export async function overlayForcePassthrough(): Promise<void> {
  return invoke('overlay_force_passthrough');
}

/** Report to Rust that this window has completed its first paint. When both
 *  overlay and toolbar have reported, Rust sizes overlay to monitor bounds
 *  and shows both windows atomically. */
export async function windowReady(label: 'overlay' | 'toolbar'): Promise<void> {
  return invoke('window_ready', { label });
}

