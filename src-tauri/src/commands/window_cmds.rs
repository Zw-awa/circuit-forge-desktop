use tauri::{Manager, WebviewWindow};

/// Set the ignore_cursor_events flag on the overlay window.
/// When true, the window becomes click-through (mouse events pass through to
/// whatever is behind it, e.g. desktop icons). When false, the window captures
/// mouse events normally.
#[tauri::command]
pub fn overlay_set_ignore_cursor(app: tauri::AppHandle, ignore: bool) -> Result<(), String> {
    let overlay: WebviewWindow = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())?;
    overlay
        .set_ignore_cursor_events(ignore)
        .map_err(|e| e.to_string())
}

/// Switch the overlay's always-on-top flag. Called when entering/leaving edit
/// mode so the overlay floats above other apps during drawing but can be
/// covered normally during click-through mode. The toolbar is re-bumped
/// afterwards so it always stays on top of the overlay.
#[tauri::command]
pub fn overlay_set_always_on_top(app: tauri::AppHandle, on_top: bool) -> Result<(), String> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        overlay
            .set_always_on_top(on_top)
            .map_err(|e| e.to_string())?;
    }
    // After any z-order change on overlay, re-assert toolbar dominance.
    if let Some(toolbar) = app.get_webview_window("toolbar") {
        let _ = toolbar.set_always_on_top(false);
        let _ = toolbar.set_always_on_top(true);
    }
    Ok(())
}

/// Resize the toolbar window. Used when expanding/collapsing side panels.
#[tauri::command]
pub fn toolbar_set_size(
    app: tauri::AppHandle,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let toolbar: WebviewWindow = app
        .get_webview_window("toolbar")
        .ok_or_else(|| "toolbar window not found".to_string())?;
    toolbar
        .set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
        .map_err(|e| e.to_string())
}

/// Move the toolbar window to a specific screen position.
#[tauri::command]
pub fn toolbar_set_position(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let toolbar: WebviewWindow = app
        .get_webview_window("toolbar")
        .ok_or_else(|| "toolbar window not found".to_string())?;
    toolbar
        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
        .map_err(|e| e.to_string())
}

/// Request overlay to quit the entire app.
#[tauri::command]
pub fn app_exit(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

/// Re-assert toolbar on top. Fix for Windows z-order: when the user clicks
/// the overlay, the overlay rises above the same-AOT toolbar. We raise the
/// toolbar back via set_focus (which calls SetForegroundWindow on Windows).
/// We deliberately do NOT toggle always_on_top here — toggling AOT on a
/// transparent window causes a DWM redraw that briefly flashes the default
/// black background.
#[tauri::command]
pub fn toolbar_bump_on_top(app: tauri::AppHandle) -> Result<(), String> {
    let toolbar: WebviewWindow = app
        .get_webview_window("toolbar")
        .ok_or_else(|| "toolbar window not found".to_string())?;
    let _ = toolbar.set_focus();
    Ok(())
}

/// Force overlay back to click-through mode. Escape hatch if the user gets
/// locked out of the taskbar/toolbar. Bound to Esc globally, plus exposed
/// through the tray menu.
/// NOTE: We deliberately do NOT touch always_on_top on either window here.
/// Changing AOT on a transparent window causes a brief DWM redraw that
/// flashes the default window background (visible as a black square).
#[tauri::command]
pub fn overlay_force_passthrough(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.set_ignore_cursor_events(true);
    }
    if let Some(toolbar) = app.get_webview_window("toolbar") {
        let _ = toolbar.set_focus();
        let _ = toolbar.emit("force-exit-edit-mode", ());
    }
    Ok(())
}
/// Called by a frontend window when its first paint is complete.
/// Once BOTH overlay and toolbar have reported ready, we size the overlay to
/// cover the primary monitor and show both windows atomically to avoid any
/// flash-of-white on startup.
#[tauri::command]
pub fn window_ready(app: tauri::AppHandle, label: String) -> Result<(), String> {
    use std::sync::Mutex;
    // Track which windows are ready using a static mutex-guarded set.
    // We only need this once at startup, so a simple bitmask is enough.
    static STATE: Mutex<u8> = Mutex::new(0);
    const READY_OVERLAY: u8 = 1 << 0;
    const READY_TOOLBAR: u8 = 1 << 1;
    const READY_BOTH: u8 = READY_OVERLAY | READY_TOOLBAR;

    let bit = match label.as_str() {
        "overlay" => READY_OVERLAY,
        "toolbar" => READY_TOOLBAR,
        _ => return Err(format!("unknown window label: {}", label)),
    };

    let mut guard = STATE.lock().map_err(|e| e.to_string())?;
    *guard |= bit;
    if *guard != READY_BOTH {
        return Ok(());
    }
    // Fall through: both windows ready, show them.
    drop(guard);

    if let Some(overlay) = app.get_webview_window("overlay") {
        // Size overlay to cover the primary monitor.
        if let Ok(Some(monitor)) = overlay.primary_monitor() {
            let size = monitor.size();
            let pos = monitor.position();
            let _ = overlay.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition { x: pos.x, y: pos.y },
            ));
            let _ = overlay.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: size.width,
                height: size.height,
            }));
        }
        // CRITICAL: make overlay click-through BEFORE showing. Overlay stays
        // always-on-top at all times (toggling AOT on a transparent window
        // flashes a black rectangle). What changes per edit-mode is only
        // whether the mouse is captured. The toolbar is also AOT; we bump
        // it after every overlay click so it stays above the overlay.
        let _ = overlay.set_ignore_cursor_events(true);
        let _ = overlay.show();
    }
    if let Some(toolbar) = app.get_webview_window("toolbar") {
        let _ = toolbar.show();
        let _ = toolbar.set_focus();
    }
    Ok(())
}

