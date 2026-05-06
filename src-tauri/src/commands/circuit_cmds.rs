use tauri::State;
use crate::EngineState;
use crate::circuit::types::ComponentKind;
use crate::circuit::wire::WireEndpoint;

#[tauri::command]
pub fn add_component(
    engine: State<'_, EngineState>,
    kind: String,
    x: f32,
    y: f32,
    plugin_id: Option<String>,
    plugin_kind_name: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    if kind == "Plugin" {
        let pid = plugin_id.ok_or("plugin_id required for Plugin kind")?;
        let kname = plugin_kind_name.ok_or("plugin_kind_name required for Plugin kind")?;

        let registration = eng.plugin_manager
            .get(&pid)
            .and_then(|p| p.registered_components.iter().find(|r| r.kind_name == kname))
            .ok_or(format!("plugin component not found: {}/{}", pid, kname))?;

        let input_offsets: Vec<(f32, f32)> = registration.input_pins.iter()
            .map(|p| (p.offset_x, p.offset_y))
            .collect();
        let output_offsets: Vec<(f32, f32)> = registration.output_pins.iter()
            .map(|p| (p.offset_x, p.offset_y))
            .collect();

        let (comp_id, input_pins, output_pins) = eng.graph.add_plugin_component(
            &pid,
            &kname,
            input_offsets,
            output_offsets,
            x,
            y,
        )?;

        let to_pin_json = |pid: &u32| {
            let pin = &eng.graph.pins[pid];
            serde_json::json!({ "id": pin.id, "offsetX": pin.offset_x, "offsetY": pin.offset_y })
        };

        return Ok(serde_json::json!({
            "componentId": comp_id,
            "inputPins": input_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
            "outputPins": output_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
        }));
    }

    let kind = match kind.as_str() {
        "And" => ComponentKind::And,
        "Or" => ComponentKind::Or,
        "Not" => ComponentKind::Not,
        "Nand" => ComponentKind::Nand,
        "Xor" => ComponentKind::Xor,
        "Switch" => ComponentKind::Switch,
        "Led" => ComponentKind::Led,
        "Button" => ComponentKind::Button,
        "Clock" => ComponentKind::Clock,
        "Random" => ComponentKind::Random,
        "Constant" => ComponentKind::Constant,
        "SevenSegment" => ComponentKind::SevenSegment,
        "Oscilloscope" => ComponentKind::Oscilloscope,
        "DelayLine" => ComponentKind::DelayLine,
        "Splitter" => ComponentKind::Splitter,
        "Merger" => ComponentKind::Merger,
        _ => return Err(format!("unknown component kind: {}", kind)),
    };

    let (comp_id, input_pins, output_pins) = eng.graph.add_component(kind, x, y)?;

    let to_pin_json = |pid: &u32| {
        let pin = &eng.graph.pins[pid];
        serde_json::json!({ "id": pin.id, "offsetX": pin.offset_x, "offsetY": pin.offset_y })
    };

    Ok(serde_json::json!({
        "componentId": comp_id,
        "inputPins": input_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
        "outputPins": output_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
    }))
}

#[tauri::command]
pub fn remove_component(
    engine: State<'_, EngineState>,
    component_id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.graph.remove_component(component_id)
}

#[tauri::command]
pub fn move_component(
    engine: State<'_, EngineState>,
    component_id: u32,
    x: f32,
    y: f32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.graph.move_component(component_id, x, y)
}

#[tauri::command]
pub fn add_wire(
    engine: State<'_, EngineState>,
    start: serde_json::Value,
    end: serde_json::Value,
    color: Option<u32>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    fn parse_endpoint(v: &serde_json::Value) -> Result<WireEndpoint, String> {
        if let Some(obj) = v.as_object() {
            if let Some(id) = obj.get("Pin").or_else(|| obj.get("pin")).and_then(|v| v.as_u64()) {
                return Ok(WireEndpoint::Pin(id as u32));
            }
            if let Some(id) = obj.get("Junction").or_else(|| obj.get("junction")).and_then(|v| v.as_u64()) {
                return Ok(WireEndpoint::Junction(id as u32));
            }
        }
        Err("invalid endpoint".into())
    }

    let start_ep = parse_endpoint(&start)?;
    let end_ep = parse_endpoint(&end)?;
    let (wire_id, net_id) = eng.graph.add_wire(start_ep, end_ep, color)?;
    Ok(serde_json::json!({
        "wireId": wire_id,
        "netId": net_id,
    }))
}

#[tauri::command]
pub fn remove_wire(
    engine: State<'_, EngineState>,
    wire_id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.graph.remove_wire(wire_id)
}

#[tauri::command]
pub fn add_junction(
    engine: State<'_, EngineState>,
    x: f32,
    y: f32,
    net_id: u32,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let id = eng.graph.add_junction(x, y, net_id);
    Ok(serde_json::json!({ "junctionId": id }))
}

#[tauri::command]
pub fn remove_junction(
    engine: State<'_, EngineState>,
    junction_id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.graph.remove_junction(junction_id)
}

#[tauri::command]
pub fn set_wire_color(
    engine: State<'_, EngineState>,
    wire_id: u32,
    color: Option<u32>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    if let Some(wire) = eng.graph.wires.get_mut(&wire_id) {
        wire.color = color;
        Ok(())
    } else {
        Err(format!("wire {} not found", wire_id))
    }
}
