mod circuit;
mod simulation;
mod project;
mod commands;
mod rules;
mod scripting;
mod verification;
mod skin;
mod packaging;
mod debugging;
mod plugins;
mod workshop;
mod keybindings;

use std::sync::{Arc, Mutex};
use simulation::engine::SimulationEngine;

type EngineState = Arc<Mutex<SimulationEngine>>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(EngineState::new(Mutex::new(SimulationEngine::new())))
        .setup(|app| {
            use tauri::{WebviewUrl, WebviewWindowBuilder, Manager, Emitter};
            use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
            use tauri::menu::{MenuBuilder, MenuItemBuilder};

            // Toolbar: the only visible piece of UI. Hidden from taskbar —
            // user accesses it via the tray icon.
            let _toolbar = WebviewWindowBuilder::new(
                app,
                "toolbar",
                WebviewUrl::App("index.html?w=toolbar".into()),
            )
            .title("CircuitForge Toolbar")
            .inner_size(360.0, 56.0)
            .position(40.0, 40.0)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .resizable(false)
            .skip_taskbar(true)
            .shadow(false)
            .visible(false)
            .build()?;

            // Overlay: hidden from taskbar AND force-hidden on startup so the
            // default white WebView2 background never flashes before the
            // transparent frontend is ready. Frontend calls window.show() after
            // first paint.
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_skip_taskbar(true);
                let _ = overlay.hide();
            }

            let show_item = MenuItemBuilder::with_id("tray-show", "显示工具栏 Show toolbar").build(app)?;
            let hide_item = MenuItemBuilder::with_id("tray-hide", "隐藏工具栏 Hide toolbar").build(app)?;
            let passthrough_item = MenuItemBuilder::with_id("tray-passthrough", "退出编辑模式 Exit edit mode").build(app)?;
            let quit_item = MenuItemBuilder::with_id("tray-quit", "退出 Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&hide_item)
                .separator()
                .item(&passthrough_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .tooltip("CircuitForge Desktop")
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "tray-show" => {
                        if let Some(w) = app.get_webview_window("toolbar") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "tray-hide" => {
                        if let Some(w) = app.get_webview_window("toolbar") {
                            let _ = w.hide();
                        }
                    }
                    "tray-passthrough" => {
                        // Escape hatch: force overlay back to click-through.
                        // Do NOT toggle AOT on either window — it flashes a
                        // black rectangle on transparent windows.
                        if let Some(overlay) = app.get_webview_window("overlay") {
                            let _ = overlay.set_ignore_cursor_events(true);
                        }
                        if let Some(toolbar) = app.get_webview_window("toolbar") {
                            let _ = toolbar.set_focus();
                            let _ = toolbar.emit("force-exit-edit-mode", ());
                        }
                    }
                    "tray-quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("toolbar") {
                            let visible = w.is_visible().unwrap_or(false);
                            if visible {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::circuit_cmds::add_component,
            commands::circuit_cmds::remove_component,
            commands::circuit_cmds::move_component,
            commands::circuit_cmds::add_wire,
            commands::circuit_cmds::remove_wire,
            commands::circuit_cmds::add_junction,
            commands::circuit_cmds::remove_junction,
            commands::circuit_cmds::set_wire_color,
            commands::simulation_cmds::toggle_switch,
            commands::simulation_cmds::sim_step,
            commands::simulation_cmds::sim_start,
            commands::simulation_cmds::sim_pause,
            commands::simulation_cmds::sim_reset,
            commands::simulation_cmds::get_signals,
            commands::simulation_cmds::press_button,
            commands::simulation_cmds::release_button,
            commands::simulation_cmds::set_constant_value,
            commands::simulation_cmds::set_component_param,
            commands::simulation_cmds::set_sim_mode,
            commands::simulation_cmds::set_tick_rate,
            commands::simulation_cmds::set_sim_speed,
            commands::simulation_cmds::sim_step_n,
            commands::simulation_cmds::get_signal_history,
            commands::simulation_cmds::get_rule_packs,
            commands::simulation_cmds::set_active_rule_pack,
            commands::simulation_cmds::create_custom_rule_pack,
            commands::simulation_cmds::delete_custom_rule_pack,
            commands::project_cmds::save_project,
            commands::project_cmds::load_project,
            commands::project_cmds::export_custom_component,
            commands::project_cmds::import_custom_component,
            commands::project_cmds::export_rule_pack,
            commands::project_cmds::import_rule_pack,
            commands::custom_cmds::create_subcircuit_def,
            commands::custom_cmds::update_subcircuit_def,
            commands::custom_cmds::delete_subcircuit_def,
            commands::custom_cmds::get_subcircuit_defs,
            commands::custom_cmds::add_subcircuit_instance,
            commands::custom_cmds::enter_subcircuit,
            commands::custom_cmds::exit_subcircuit,
            commands::custom_cmds::get_lua_component_defs,
            commands::custom_cmds::create_lua_component_def,
            commands::custom_cmds::update_lua_component_def,
            commands::custom_cmds::delete_lua_component_def,
            commands::custom_cmds::add_lua_component_instance,
            commands::custom_cmds::validate_lua_script,
            commands::custom_cmds::create_truth_table,
            commands::custom_cmds::update_truth_table,
            commands::custom_cmds::delete_truth_table,
            commands::custom_cmds::get_truth_table,
            commands::custom_cmds::verify_truth_table_cmd,
            commands::skin_cmds::load_skin_pack,
            commands::skin_cmds::get_active_skin,
            commands::skin_cmds::set_active_skin,
            commands::skin_cmds::get_skin_asset,
            commands::skin_cmds::clear_skin,
            commands::skin_cmds::export_skin_pack,
            commands::packaging_cmds::export_circuitforge,
            commands::packaging_cmds::import_circuitforge,
            commands::packaging_cmds::create_snapshot_cmd,
            commands::packaging_cmds::list_snapshots,
            commands::packaging_cmds::restore_snapshot,
            commands::debug_cmds::add_breakpoint,
            commands::debug_cmds::remove_breakpoint,
            commands::debug_cmds::list_breakpoints,
            commands::debug_cmds::set_breakpoint_enabled,
            commands::debug_cmds::debug_step_into,
            commands::debug_cmds::debug_step_over,
            commands::debug_cmds::debug_continue,
            commands::debug_cmds::get_bulk_signal_history,
            commands::debug_cmds::export_waveform_csv,
            commands::plugin_cmds::plugin_load,
            commands::plugin_cmds::plugin_unload,
            commands::plugin_cmds::plugin_list,
            commands::plugin_cmds::plugin_set_enabled,
            commands::plugin_cmds::plugin_get_components,
            commands::plugin_cmds::plugin_get_menu_items,
            commands::plugin_cmds::plugin_get_export_formats,
            commands::plugin_cmds::plugin_call_menu_item,
            commands::plugin_cmds::plugin_evaluate,
            commands::keybinding_cmds::get_keybindings,
            commands::keybinding_cmds::set_keybinding,
            commands::keybinding_cmds::reset_keybindings,
            commands::keybinding_cmds::export_keybindings,
            commands::keybinding_cmds::import_keybindings,
            commands::workshop_cmds::workshop_fetch_index,
            commands::workshop_cmds::workshop_download_item,
            commands::workshop_cmds::workshop_search,
            commands::window_cmds::overlay_set_ignore_cursor,
            commands::window_cmds::overlay_set_always_on_top,
            commands::window_cmds::toolbar_set_size,
            commands::window_cmds::toolbar_set_position,
            commands::window_cmds::toolbar_bump_on_top,
            commands::window_cmds::overlay_force_passthrough,
            commands::window_cmds::app_exit,
            commands::window_cmds::window_ready,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
