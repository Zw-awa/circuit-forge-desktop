use crate::simulation::engine::SimulationEngine;
use crate::simulation::evaluator::evaluate_gate;
use crate::circuit::types::{ComponentKind, Signal, NetId, SignalType};
use crate::circuit::graph::CircuitGraph;
use crate::scripting::sandbox::LuaSandbox;
use super::truth_table::{TruthTable, VerificationResult, VerificationFailure, TargetType};
use std::collections::HashMap;

pub fn verify_truth_table(
    engine: &mut SimulationEngine,
    table: &TruthTable,
) -> VerificationResult {
    let total_rows = table.rows.len();
    let mut passed_rows = 0;
    let mut failures: Vec<VerificationFailure> = Vec::new();

    match table.target_type {
        TargetType::LuaScript => {
            let sandbox = match LuaSandbox::new() {
                Ok(s) => s,
                Err(_) => {
                    return VerificationResult {
                        passed: false,
                        total_rows,
                        passed_rows: 0,
                        failures: vec![VerificationFailure {
                            row_index: 0,
                            inputs: vec![],
                            expected: vec![],
                            actual: vec![],
                        }],
                    };
                }
            };
            if let Some(_def) = engine.lua_registry.get(table.target_def_id) {
                let source = &_def.script_source;
                let empty_state = serde_json::json!({});
                for (row_idx, row) in table.rows.iter().enumerate() {
                    match sandbox.evaluate(source, &row.inputs, &empty_state, false) {
                        Ok((actual_outputs, _)) => {
                            let mut mismatch = false;
                            for (i, expected) in row.expected_outputs.iter().enumerate() {
                                let actual = actual_outputs.get(i).copied().unwrap_or(Signal::Low);
                                if &actual != expected {
                                    mismatch = true;
                                }
                            }
                            if row.expected_outputs.len() != actual_outputs.len() {
                                mismatch = true;
                            }
                            if mismatch {
                                failures.push(VerificationFailure {
                                    row_index: row_idx,
                                    inputs: row.inputs.clone(),
                                    expected: row.expected_outputs.clone(),
                                    actual: actual_outputs,
                                });
                            } else {
                                passed_rows += 1;
                            }
                        }
                Err(_e) => {
                            failures.push(VerificationFailure {
                                row_index: row_idx,
                                inputs: row.inputs.clone(),
                                expected: row.expected_outputs.clone(),
                                actual: vec![],
                            });
                            eprintln!("Lua eval error at row {}: {}", row_idx, _e);
                        }
                    }
                }
            } else {
                failures.push(VerificationFailure {
                    row_index: 0,
                    inputs: vec![],
                    expected: vec![],
                    actual: vec![],
                });
            }
        }
        TargetType::SubCircuit => {
            if let Some(def) = engine.subcircuit_registry.get(table.target_def_id) {
                let def = def.clone();
                // Map external input pins by name → internal_pin_id
                let input_pins: Vec<(String, u32)> = def.external_pins.iter()
                    .filter(|ep| !ep.is_output)
                    .map(|ep| (ep.name.clone(), ep.internal_pin_id))
                    .collect();

                // Map external output pins by name → internal_pin_id
                let output_pins_map: Vec<(String, u32)> = def.external_pins.iter()
                    .filter(|ep| ep.is_output)
                    .map(|ep| (ep.name.clone(), ep.internal_pin_id))
                    .collect();

                let signal_type = engine.rule_registry.active().signal_type;
                for (row_idx, row) in table.rows.iter().enumerate() {
                    // Simulate the inner graph for this row of inputs
                    let actual = simulate_inner_graph(
                        &def.inner_graph,
                        &input_pins,
                        &output_pins_map,
                        &row.inputs,
                        &signal_type,
                        &engine.subcircuit_registry,
                        &engine.lua_registry,
                    );

                    let mut mismatch = false;
                    for (i, expected) in row.expected_outputs.iter().enumerate() {
                        let actual_val = actual.get(i).copied().unwrap_or(Signal::Low);
                        if &actual_val != expected {
                            mismatch = true;
                        }
                    }
                    if row.expected_outputs.len() != actual.len() {
                        mismatch = true;
                    }

                    if mismatch {
                        failures.push(VerificationFailure {
                            row_index: row_idx,
                            inputs: row.inputs.clone(),
                            expected: row.expected_outputs.clone(),
                            actual,
                        });
                    } else {
                        passed_rows += 1;
                    }
                }
            } else {
                failures.push(VerificationFailure {
                    row_index: 0,
                    inputs: vec![],
                    expected: vec![],
                    actual: vec![],
                });
            }
        }
    }

    VerificationResult {
        passed: failures.is_empty() && total_rows > 0,
        total_rows,
        passed_rows,
        failures,
    }
}

fn simulate_inner_graph(
    graph: &CircuitGraph,
    input_pins: &[(String, u32)],
    output_pins: &[(String, u32)],
    inputs: &[Signal],
    signal_type: &SignalType,
    subcircuit_registry: &crate::circuit::subcircuit::SubCircuitDefRegistry,
    lua_registry: &crate::scripting::lua_engine::LuaComponentDefRegistry,
) -> Vec<Signal> {
    // Build a temporary signal map for the inner graph
    let mut signals: HashMap<NetId, Signal> = HashMap::new();

    // Map table row inputs to input nets based on position (order in external_pins)
    for (i, (_name, pin_id)) in input_pins.iter().enumerate() {
        if i < inputs.len() {
            if let Some(pin) = graph.pins.get(pin_id) {
                if let Some(net_id) = pin.net {
                    signals.insert(net_id, inputs[i]);
                }
            }
        }
    }

    // Collect all components
    let comp_ids: Vec<u32> = graph.components.keys().copied().collect();

    // Propagate until steady state (no changes detected), max 100 iterations
    let mut iteration = 0;
    loop {
        let mut changed = false;
        for &comp_id in &comp_ids {
            if let Some(comp) = graph.components.get(&comp_id) {
                match comp.kind.clone() {
                    ComponentKind::And | ComponentKind::Or | ComponentKind::Not
                    | ComponentKind::Nand | ComponentKind::Xor => {
                        let comp_inputs: Vec<Signal> = comp.input_pins.iter()
                            .filter_map(|pid| graph.pins.get(pid))
                            .filter_map(|p| p.net)
                            .filter_map(|n| signals.get(&n).copied())
                            .collect();
                        if comp_inputs.len() == comp.input_pins.len() {
                            let output = evaluate_gate(comp.kind.clone(), &comp_inputs, signal_type);
                            for out_pin_id in &comp.output_pins {
                                if let Some(pin) = graph.pins.get(out_pin_id) {
                                    if let Some(net_id) = pin.net {
                                        let old = signals.get(&net_id).copied().unwrap_or(Signal::Low);
                                        if old != output {
                                            signals.insert(net_id, output);
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ComponentKind::Switch | ComponentKind::Button => {
                        let toggle = comp.toggle_state.unwrap_or(Signal::Low);
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    signals.insert(net_id, toggle);
                                }
                            }
                        }
                    }
                    ComponentKind::Constant => {
                        let val = comp.constant_value.unwrap_or(Signal::Low);
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    signals.insert(net_id, val);
                                }
                            }
                        }
                    }
                    ComponentKind::Clock => {
                        let period = comp.clock_period.unwrap_or(2);
                        let counter = comp.clock_counter.unwrap_or(0);
                        let high_ticks = (period as f32 * comp.clock_duty.unwrap_or(0.5)) as u32;
                        let out = if counter < high_ticks { Signal::High } else { Signal::Low };
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    signals.insert(net_id, out);
                                }
                            }
                        }
                    }
                    ComponentKind::Random => {
                        let prob = comp.random_probability.unwrap_or(0.5);
                        let val = (comp.id as f32 * 0.37).fract();
                        let out = if val < prob { Signal::High } else { Signal::Low };
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    signals.insert(net_id, out);
                                }
                            }
                        }
                    }
                    ComponentKind::DelayLine => {
                        let input = comp.input_pins.iter()
                            .filter_map(|pid| graph.pins.get(pid))
                            .filter_map(|p| p.net)
                            .filter_map(|n| signals.get(&n).copied())
                            .next()
                            .unwrap_or(Signal::Low);
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    signals.insert(net_id, input);
                                }
                            }
                        }
                    }
                    ComponentKind::Splitter => {
                        let input = comp.input_pins.iter()
                            .filter_map(|pid| graph.pins.get(pid))
                            .filter_map(|p| p.net)
                            .filter_map(|n| signals.get(&n).copied())
                            .next()
                            .unwrap_or(Signal::Low);
                        let value = match input {
                            Signal::Bus(v) => v,
                            Signal::High => 1,
                            Signal::Low => 0,
                            Signal::Integer(v) => v as u8,
                            Signal::Float(v) => v.round() as u8,
                        };
                        for (i, out_pin_id) in comp.output_pins.iter().enumerate() {
                            let bit = if (value >> i) & 1 == 1 { Signal::High } else { Signal::Low };
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    signals.insert(net_id, bit);
                                }
                            }
                        }
                    }
                    ComponentKind::Merger => {
                        let mut value: u8 = 0;
                        for (i, in_pin_id) in comp.input_pins.iter().enumerate() {
                            let sig = graph.pins.get(in_pin_id)
                                .and_then(|p| p.net)
                                .and_then(|n| signals.get(&n).copied())
                                .unwrap_or(Signal::Low);
                            if sig == Signal::High {
                                value |= 1 << i;
                            }
                        }
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    signals.insert(net_id, Signal::Bus(value));
                                }
                            }
                        }
                    }
                    ComponentKind::SubCircuit(inner_def_id) => {
                        if let Some(inner_def) = subcircuit_registry.get(inner_def_id) {
                            let inner_def = inner_def.clone();
                            let mut inner_input_pins: Vec<(String, u32)> = Vec::new();
                            let mut inner_output_pins: Vec<(String, u32)> = Vec::new();
                            let mut ext_inputs: Vec<Signal> = Vec::new();
                            for ext_pin in &inner_def.external_pins {
                                if ext_pin.is_output {
                                    inner_output_pins.push((ext_pin.name.clone(), ext_pin.internal_pin_id));
                                } else {
                                    let inner_pin = inner_def.inner_graph.pins.get(&ext_pin.internal_pin_id);
                                    let sig = inner_pin
                                        .and_then(|p| p.net)
                                        .and_then(|nid| signals.get(&nid).copied())
                                        .unwrap_or(Signal::Low);
                                    inner_input_pins.push((ext_pin.name.clone(), ext_pin.internal_pin_id));
                                    ext_inputs.push(sig);
                                }
                            }
                            let inner_actual = simulate_inner_graph(
                                &inner_def.inner_graph,
                                &inner_input_pins,
                                &inner_output_pins,
                                &ext_inputs,
                                signal_type,
                                subcircuit_registry,
                                lua_registry,
                            );
                            // For verification, subcircuits are evaluated in isolation — propagate their outputs
                            for (i, out_pin_id) in comp.output_pins.iter().enumerate() {
                                let out = inner_actual.get(i).copied().unwrap_or(Signal::Low);
                                if let Some(pin) = graph.pins.get(out_pin_id) {
                                    if let Some(net_id) = pin.net {
                                        signals.insert(net_id, out);
                                    }
                                }
                            }
                        }
                    }
                    ComponentKind::LuaScript(inner_def_id) => {
                        if let Some(lua_def) = lua_registry.get(inner_def_id) {
                            let lua_def = lua_def.clone();
                            let lua_inputs: Vec<Signal> = comp.input_pins.iter()
                                .filter_map(|pid| graph.pins.get(pid))
                                .filter_map(|p| p.net)
                                .filter_map(|n| signals.get(&n).copied())
                                .collect();
                            match LuaSandbox::new() {
                                Ok(sandbox) => {
                                    let state = comp.lua_state.clone().unwrap_or(serde_json::json!({}));
                                    match sandbox.evaluate(&lua_def.script_source, &lua_inputs, &state, false) {
                                        Ok((outputs, _new_state)) => {
                                            for (i, out_pin_id) in comp.output_pins.iter().enumerate() {
                                                let out = outputs.get(i).copied().unwrap_or(Signal::Low);
                                                if let Some(pin) = graph.pins.get(out_pin_id) {
                                                    if let Some(net_id) = pin.net {
                                                        signals.insert(net_id, out);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => { eprintln!("Verifier Lua eval error: {}", e); }
                                    }
                                }
                                Err(e) => { eprintln!("Verifier Lua sandbox error: {}", e); }
                            }
                        }
                    }
                    ComponentKind::Led | ComponentKind::SevenSegment | ComponentKind::Oscilloscope
                    | ComponentKind::Plugin(_, _) => {}
                }
            }
        }
        if !changed {
            break;
        }
        iteration += 1;
        if iteration >= 100 {
            break;
        }
    }

    // Collect output signals in the order of output_pins
    let mut result = Vec::new();
    for (_name, pin_id) in output_pins {
        if let Some(pin) = graph.pins.get(pin_id) {
            if let Some(net_id) = pin.net {
                let sig = signals.get(&net_id).copied().unwrap_or(Signal::Low);
                result.push(sig);
            } else {
                result.push(Signal::Low);
            }
        } else {
            result.push(Signal::Low);
        }
    }
    result
}
