use tauri::State;
use crate::EngineState;
use crate::debugging::breakpoint::{BreakpointTarget, BreakpointCondition};
use crate::simulation::engine::StepResult;

fn parse_target(v: &serde_json::Value) -> Result<BreakpointTarget, String> {
    if let Some(net) = v.get("Net").and_then(|n| n.as_u64()) {
        Ok(BreakpointTarget::Net(net as u32))
    } else if let Some(comp) = v.get("Component").and_then(|c| c.as_u64()) {
        Ok(BreakpointTarget::Component(comp as u32))
    } else {
        Err("invalid breakpoint target: must be {{\"Net\": N}} or {{\"Component\": N}}".into())
    }
}

fn parse_condition(v: &serde_json::Value) -> Result<BreakpointCondition, String> {
    if let Some(s) = v.as_str() {
        match s {
            "SignalChanges" => return Ok(BreakpointCondition::SignalChanges),
            "RisingEdge" => return Ok(BreakpointCondition::RisingEdge),
            "FallingEdge" => return Ok(BreakpointCondition::FallingEdge),
            _ => {}
        }
    }
    if let Some(sig) = v.get("SignalEquals") {
        let signal: crate::circuit::types::Signal = serde_json::from_value(sig.clone()).map_err(|e| e.to_string())?;
        return Ok(BreakpointCondition::SignalEquals(signal));
    }
    Err("invalid breakpoint condition: must be \"SignalChanges\", \"RisingEdge\", \"FallingEdge\", or {{\"SignalEquals\": <signal>}}".into())
}

#[tauri::command]
pub fn add_breakpoint(
    engine: State<'_, EngineState>,
    target: serde_json::Value,
    condition: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let target = parse_target(&target)?;
    let condition = parse_condition(&condition)?;
    let graph = eng.graph.clone();
    let id = eng.breakpoint_manager.add(target, condition, true, &graph);
    Ok(serde_json::json!({ "id": id }))
}

#[tauri::command]
pub fn remove_breakpoint(
    engine: State<'_, EngineState>,
    id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.breakpoint_manager.remove(id);
    Ok(())
}

#[tauri::command]
pub fn list_breakpoints(
    engine: State<'_, EngineState>,
) -> Result<Vec<serde_json::Value>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let bps: Vec<_> = eng.breakpoint_manager.list()
        .into_iter()
        .map(|bp| {
            serde_json::json!({
                "id": bp.id,
                "target": match &bp.target {
                    BreakpointTarget::Net(n) => serde_json::json!({ "Net": n }),
                    BreakpointTarget::Component(c) => serde_json::json!({ "Component": c }),
                },
                "condition": match &bp.condition {
                    BreakpointCondition::SignalEquals(s) => serde_json::json!({ "SignalEquals": s }),
                    BreakpointCondition::SignalChanges => serde_json::json!("SignalChanges"),
                    BreakpointCondition::RisingEdge => serde_json::json!("RisingEdge"),
                    BreakpointCondition::FallingEdge => serde_json::json!("FallingEdge"),
                },
                "enabled": bp.enabled,
            })
        })
        .collect();
    Ok(bps)
}

#[tauri::command]
pub fn set_breakpoint_enabled(
    engine: State<'_, EngineState>,
    id: u32,
    enabled: bool,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.breakpoint_manager.set_enabled(id, enabled);
    Ok(())
}

#[tauri::command]
pub fn debug_step_into(
    engine: State<'_, EngineState>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let result = eng.step_single_event()?;
    step_result_to_json(result)
}

#[tauri::command]
pub fn debug_step_over(
    engine: State<'_, EngineState>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let (changed, bp_hit) = eng.propagate();
    let events_remaining = eng.event_queue.len();
    Ok(serde_json::json!({
        "changed": changed.iter().map(|(k, v)| {
            (k.to_string(), crate::commands::simulation_cmds::signal_to_json_value(v))
        }).collect::<std::collections::HashMap<_, _>>(),
        "breakpointHit": bp_hit.map(|h| serde_json::json!({
            "breakpointId": h.breakpoint_id,
            "netId": h.net_id,
            "oldSignal": crate::commands::simulation_cmds::signal_to_json_value(&h.old_signal),
            "newSignal": crate::commands::simulation_cmds::signal_to_json_value(&h.new_signal),
        })),
        "eventsRemaining": events_remaining,
    }))
}

#[tauri::command]
pub fn debug_continue(
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.debug_continue();
    Ok(())
}

fn step_result_to_json(result: StepResult) -> Result<serde_json::Value, String> {
    let changed_map: std::collections::HashMap<String, serde_json::Value> = result.changed
        .iter()
        .map(|(k, v)| {
            (k.to_string(), crate::commands::simulation_cmds::signal_to_json_value(v))
        })
        .collect();
    Ok(serde_json::json!({
        "changed": changed_map,
        "breakpointHit": result.breakpoint_hit.map(|h| serde_json::json!({
            "breakpointId": h.breakpoint_id,
            "netId": h.net_id,
            "oldSignal": crate::commands::simulation_cmds::signal_to_json_value(&h.old_signal),
            "newSignal": crate::commands::simulation_cmds::signal_to_json_value(&h.new_signal),
        })),
        "eventsRemaining": result.events_remaining,
    }))
}

#[tauri::command]
pub fn get_bulk_signal_history(
    engine: State<'_, EngineState>,
    net_ids: Vec<u32>,
    from_tick: Option<u64>,
    to_tick: Option<u64>,
) -> Result<serde_json::Value, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let result = eng.get_bulk_signal_history(&net_ids, from_tick, to_tick);
    let json_map: serde_json::Map<String, serde_json::Value> = result
        .into_iter()
        .map(|(net_id, data)| {
            let entries: Vec<serde_json::Value> = data
                .into_iter()
                .map(|(tick, signal)| {
                    serde_json::json!([tick, crate::commands::simulation_cmds::signal_to_json_value(&signal)])
                })
                .collect();
            (net_id.to_string(), serde_json::Value::Array(entries))
        })
        .collect();
    Ok(serde_json::Value::Object(json_map))
}

#[tauri::command]
pub fn export_waveform_csv(
    engine: State<'_, EngineState>,
    net_ids: Vec<u32>,
) -> Result<String, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    eng.export_waveform_csv(&net_ids)
}
