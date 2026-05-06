use std::collections::HashMap;
use tauri::{Emitter, State, AppHandle};
use crate::EngineState;
use crate::circuit::types::Signal;
use crate::circuit::types::SimMode;
use crate::rules::presets::RulePack;
use crate::simulation::engine::{SimStatus, SimEvent};
use crate::debugging::breakpoint::BreakpointHitInfo;

fn propagate_result_to_json(changed: &HashMap<u32, Signal>, bp_hit: &Option<BreakpointHitInfo>) -> serde_json::Value {
    let mut result = signals_to_json(changed);
    if let Some(hit) = bp_hit {
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert("breakpointHit".into(), serde_json::json!({
                "breakpointId": hit.breakpoint_id,
                "netId": hit.net_id,
                "oldSignal": signal_to_json_value(&hit.old_signal),
                "newSignal": signal_to_json_value(&hit.new_signal),
                "tick": hit.tick,
            }));
        }
    }
    result
}

pub fn signal_to_json_value(signal: &Signal) -> serde_json::Value {
    match signal {
        Signal::High => serde_json::json!("High"),
        Signal::Low => serde_json::json!("Low"),
        Signal::Bus(n) => serde_json::json!({ "Bus": n }),
        Signal::Integer(n) => serde_json::json!({ "Integer": n }),
        Signal::Float(n) => serde_json::json!({ "Float": n }),
    }
}

fn signals_to_json(signals: &HashMap<u32, Signal>) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = signals
        .iter()
        .map(|(k, v)| (k.to_string(), signal_to_json_value(v)))
        .collect();
    serde_json::Value::Object(map)
}

#[tauri::command]
pub fn toggle_switch(
    engine: State<'_, EngineState>,
    component_id: u32,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let changed = eng.toggle_switch(component_id)?;
    Ok(signals_to_json(&changed))
}

#[tauri::command]
pub fn sim_step(
    engine: State<'_, EngineState>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let (changed, bp_hit) = eng.step();
    Ok(propagate_result_to_json(&changed, &bp_hit))
}

#[tauri::command]
pub fn sim_start(
    engine: State<'_, EngineState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut eng = engine.lock().map_err(|e| e.to_string())?;
        if eng.status == SimStatus::Running {
            return Err("simulation already running".into());
        }
        eng.status = SimStatus::Running;
    }

    let engine_clone = std::sync::Arc::clone(&engine);
    std::thread::spawn(move || loop {
        let mut eng = engine_clone.lock().unwrap();
        if eng.status != SimStatus::Running {
            break;
        }

        let (changed, bp_hit) = match eng.sim_mode {
            SimMode::TickDriven => {
                eng.tick_driven_step()
            }
            SimMode::EventDriven => {
                eng.advance_clocks();
                eng.advance_randoms();
                eng.advance_delay_lines();
                eng.propagate()
            }
        };

        let tick = eng.tick_count;
        let interval_ms = match eng.sim_mode {
            SimMode::TickDriven => {
                (1000.0 / (eng.tick_rate as f32 * eng.speed_multiplier)) as u64
            }
            SimMode::EventDriven => {
                let raw = (1000.0 / eng.tick_rate as f32) / eng.speed_multiplier;
                (raw as u64).max(1)
            }
        };
        drop(eng);

        if !changed.is_empty() {
            let json_changed: serde_json::Map<_, _> = changed
                .iter()
                .map(|(k, v)| (k.to_string(), signal_to_json_value(v)))
                .collect();
            let _ = app.emit(
                "sim-tick",
                serde_json::json!({
                    "tick": tick,
                    "changed": json_changed,
                }),
            );
        }

        if let Some(hit) = bp_hit {
            let hit_json = serde_json::json!({
                "breakpointId": hit.breakpoint_id,
                "netId": hit.net_id,
                "oldSignal": signal_to_json_value(&hit.old_signal),
                "newSignal": signal_to_json_value(&hit.new_signal),
            });
            let _ = app.emit("breakpoint-hit", hit_json);
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
    });
    Ok(())
}

#[tauri::command]
pub fn sim_pause(
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.status = SimStatus::Paused;
    Ok(())
}

#[tauri::command]
pub fn sim_reset(
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.reset();
    Ok(())
}

#[tauri::command]
pub fn get_signals(
    engine: State<'_, EngineState>,
) -> Result<serde_json::Value, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let signals = eng.get_signals();
    Ok(signals_to_json(signals))
}

#[tauri::command]
pub fn press_button(
    engine: State<'_, EngineState>,
    component_id: u32,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let comp = eng
        .graph
        .components
        .get_mut(&component_id)
        .ok_or("not found")?;
    comp.press_state = Some(true);
    let out_pins = comp.output_pins.clone();
    for pin_id in out_pins {
        if let Some(pin) = eng.graph.pins.get(&pin_id) {
            if let Some(net_id) = pin.net {
                eng.event_queue.push_back(SimEvent {
                    net_id,
                    new_signal: Signal::High,
                });
            }
        }
    }
    let (changed, bp_hit) = eng.propagate();
    Ok(propagate_result_to_json(&changed, &bp_hit))
}

#[tauri::command]
pub fn release_button(
    engine: State<'_, EngineState>,
    component_id: u32,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let comp = eng
        .graph
        .components
        .get_mut(&component_id)
        .ok_or("not found")?;
    comp.press_state = Some(false);
    let out_pins = comp.output_pins.clone();
    for pin_id in out_pins {
        if let Some(pin) = eng.graph.pins.get(&pin_id) {
            if let Some(net_id) = pin.net {
                eng.event_queue.push_back(SimEvent {
                    net_id,
                    new_signal: Signal::Low,
                });
            }
        }
    }
    let (changed, bp_hit) = eng.propagate();
    Ok(propagate_result_to_json(&changed, &bp_hit))
}

#[tauri::command]
pub fn set_constant_value(
    engine: State<'_, EngineState>,
    component_id: u32,
    value: String,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let comp = eng
        .graph
        .components
        .get_mut(&component_id)
        .ok_or("not found")?;
    let signal = match value.as_str() {
        "High" => Signal::High,
        "Low" => Signal::Low,
        s if s.starts_with("Bus(") => {
            let v: u8 = s[4..s.len() - 1]
                .parse()
                .map_err(|_| "invalid bus value")?;
            Signal::Bus(v)
        }
        _ => return Err("invalid signal value".into()),
    };
    comp.constant_value = Some(signal);
    let out_pins = comp.output_pins.clone();
    for pin_id in out_pins {
        if let Some(pin) = eng.graph.pins.get(&pin_id) {
            if let Some(net_id) = pin.net {
                eng.event_queue.push_back(SimEvent {
                    net_id,
                    new_signal: signal,
                });
            }
        }
    }
    let (changed, bp_hit) = eng.propagate();
    Ok(propagate_result_to_json(&changed, &bp_hit))
}

#[tauri::command]
pub fn set_component_param(
    engine: State<'_, EngineState>,
    component_id: u32,
    param: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let comp = eng
        .graph
        .components
        .get_mut(&component_id)
        .ok_or("not found")?;
    match param.as_str() {
        "clock_period" => comp.clock_period = value.as_u64().map(|v| v as u32),
        "clock_duty" => comp.clock_duty = value.as_f64().map(|v| v as f32),
        "delay_ticks" => comp.delay_ticks = value.as_u64().map(|v| v as u32),
        "bus_width" => comp.bus_width = value.as_u64().map(|v| v as u32),
            "random_probability" => comp.random_probability = value.as_f64().map(|v| v as f32),
            "oscilloscope_channels" => comp.oscilloscope_channels = value.as_u64().map(|v| v as u32),
            "oscilloscope_time_window" => comp.oscilloscope_time_window = value.as_u64().map(|v| v as u32),
            "constant_value" => {
            if let Some(s) = value.as_str() {
                comp.constant_value = match s {
                    "High" => Some(Signal::High),
                    "Low" => Some(Signal::Low),
                    _ => None,
                };
            }
        }
        _ => return Err(format!("unknown param: {}", param)),
    }
    Ok(())
}

#[tauri::command]
pub fn set_sim_mode(
    engine: State<'_, EngineState>,
    mode: String,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let sim_mode = match mode.as_str() {
        "event" => SimMode::EventDriven,
        "tick" => SimMode::TickDriven,
        _ => return Err("invalid mode".into()),
    };
    eng.set_mode(sim_mode);
    Ok(())
}

#[tauri::command]
pub fn set_tick_rate(
    engine: State<'_, EngineState>,
    rate: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.tick_rate = rate;
    Ok(())
}

#[tauri::command]
pub fn set_sim_speed(
    engine: State<'_, EngineState>,
    multiplier: f32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.speed_multiplier = multiplier;
    Ok(())
}

#[tauri::command]
pub fn sim_step_n(
    engine: State<'_, EngineState>,
    n: u32,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let mut all_changed: HashMap<u32, Signal> = HashMap::new();
    let mut last_bp_hit: Option<BreakpointHitInfo> = None;
    for _ in 0..n {
        let (changed, bp_hit) = eng.step();
        all_changed.extend(changed);
        if bp_hit.is_some() {
            last_bp_hit = bp_hit;
            break;
        }
    }
    Ok(propagate_result_to_json(&all_changed, &last_bp_hit))
}

#[tauri::command]
pub fn get_signal_history(
    engine: State<'_, EngineState>,
    net_id: u32,
) -> Result<serde_json::Value, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let history = eng.signal_history.get(&net_id);
    match history {
        Some(h) => {
            let data: Vec<serde_json::Value> = h
                .get_data()
                .iter()
                .map(|(tick, signal)| {
                    serde_json::json!({
                        "tick": tick,
                        "signal": match signal {
                            Signal::High => serde_json::json!("High"),
                            Signal::Low => serde_json::json!("Low"),
                            Signal::Bus(v) => serde_json::json!(format!("Bus({})", v)),
                            Signal::Integer(v) => serde_json::json!(format!("Integer({})", v)),
                            Signal::Float(v) => serde_json::json!(format!("Float({})", v)),
                        }
                    })
                })
                .collect();
            Ok(serde_json::Value::Array(data))
        }
        None => Ok(serde_json::Value::Array(vec![])),
    }
}

#[tauri::command]
pub fn get_rule_packs(
    engine: State<'_, EngineState>,
) -> Result<Vec<RulePack>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    Ok(eng.rule_registry.list_all().into_iter().cloned().collect())
}

#[tauri::command]
pub fn set_active_rule_pack(
    engine: State<'_, EngineState>,
    id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.rule_registry.set_active(id)
}

#[tauri::command]
pub fn create_custom_rule_pack(
    engine: State<'_, EngineState>,
    pack_json: serde_json::Value,
) -> Result<u32, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let pack: RulePack = serde_json::from_value(pack_json).map_err(|e| e.to_string())?;
    Ok(eng.rule_registry.add_custom(pack))
}

#[tauri::command]
pub fn delete_custom_rule_pack(
    engine: State<'_, EngineState>,
    id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.rule_registry.remove_custom(id)
}
