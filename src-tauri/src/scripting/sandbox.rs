use std::cell::Cell;
use mlua::{Lua, Function, Table, Value, MultiValue, VmState, HookTriggers, LuaSerdeExt};
use crate::circuit::types::Signal;

const MAX_MEMORY_MB: usize = 16;
const MAX_INSTRUCTIONS: u64 = 1_000_000;

pub struct LuaSandbox {
    lua: Lua,
}

impl LuaSandbox {
    pub fn new() -> Result<Self, String> {
        let lua = Lua::new();
        lua.globals().set("io", Value::Nil).map_err(|e| e.to_string())?;
        lua.globals().set("os", Value::Nil).map_err(|e| e.to_string())?;
        lua.globals().set("debug", Value::Nil).map_err(|e| e.to_string())?;
        lua.globals().set("loadfile", Value::Nil).map_err(|e| e.to_string())?;
        lua.globals().set("dofile", Value::Nil).map_err(|e| e.to_string())?;
        lua.globals().set("require", Value::Nil).map_err(|e| e.to_string())?;
        lua.globals().set("rawget", Value::Nil).map_err(|e| e.to_string())?;
        lua.globals().set("rawset", Value::Nil).map_err(|e| e.to_string())?;

        let print_fn = lua.create_function(|_, args: MultiValue| {
            let msg: Vec<String> = args.into_iter().map(|v| format!("{:?}", v)).collect();
            eprintln!("[Lua] {}", msg.join("\t"));
            Ok(Value::Nil)
        }).map_err(|e| e.to_string())?;
        lua.globals().set("print", print_fn).map_err(|e| e.to_string())?;

        lua.set_memory_limit(MAX_MEMORY_MB * 1024 * 1024).map_err(|e| e.to_string())?;
        let total = Cell::new(0u64);
        let batch: u64 = 100_000;
        lua.set_hook(
            HookTriggers::default().every_nth_instruction(batch as u32),
            move |_lua, _debug| {
                let t = total.get() + batch;
                total.set(t);
                if t >= MAX_INSTRUCTIONS {
                    return Err(mlua::Error::external("instruction limit exceeded"));
                }
                Ok(VmState::Continue)
            },
        ).map_err(|e| e.to_string())?;
        Ok(Self { lua })
    }

    pub fn evaluate(
        &self,
        script: &str,
        inputs: &[Signal],
        state: &serde_json::Value,
        script_loaded: bool,
    ) -> Result<(Vec<Signal>, serde_json::Value), String> {
        if !script_loaded {
            self.lua.load(script).exec().map_err(|e| format!("Lua error: {}", e))?;
        }

        let evaluate_fn: Function = self.lua.globals()
            .get("evaluate")
            .map_err(|e| format!("No evaluate function: {}", e))?;

        let input_table = self.lua.create_table().map_err(|e| e.to_string())?;
        for (i, signal) in inputs.iter().enumerate() {
            let val: f64 = match signal {
                Signal::Low => 0.0,
                Signal::High => 1.0,
                Signal::Bus(v) => *v as f64,
                Signal::Integer(v) => *v as f64,
                Signal::Float(v) => *v,
            };
            input_table.set(i + 1, val).map_err(|e| e.to_string())?;
        }

        let state_str = serde_json::to_string(state).map_err(|e| e.to_string())?;
        let state_value = self.lua.create_string(&state_str)
            .map_err(|e| e.to_string())?;

        let result = evaluate_fn.call::<MultiValue>((input_table, state_value))
            .map_err(|e| format!("Lua runtime: {}", e))?;

        let mut iter = result.into_iter();
        let outputs_table: Table = match iter.next() {
            Some(Value::Table(t)) => t,
            Some(v) => return Err(format!("expected table as first return value, got: {:?}", v)),
            None => return Err("no outputs".into()),
        };
        let new_state_value: Value = iter.next().unwrap_or(Value::Nil);

        let serde_state = match &new_state_value {
            Value::String(s) => {
                let state_str = s.to_str().map_err(|e| e.to_string())?;
                serde_json::from_str(&state_str).map_err(|e| e.to_string())?
            }
            Value::Nil => serde_json::json!({}),
            Value::Table(t) => {
                let json_str = serde_json::to_string(
                    &self.lua.from_value::<serde_json::Value>(Value::Table(t.clone()))
                        .map_err(|e| format!("state table conversion: {}", e))?
                ).map_err(|e| e.to_string())?;
                serde_json::from_str(&json_str).map_err(|e| e.to_string())?
            }
            other => return Err(format!("expected string, table, or nil as state, got: {:?}", other)),
        };

        let mut outputs = Vec::new();
        let len = outputs_table.raw_len() as usize;
        for i in 0..len {
            let val: f64 = outputs_table.get(i + 1).unwrap_or(0.0);
            outputs.push(if val == 0.0 {
                Signal::Low
            } else if val == 1.0 {
                Signal::High
            } else if val.fract() == 0.0 {
                Signal::Integer(val as i32)
            } else {
                Signal::Float(val)
            });
        }

        Ok((outputs, serde_state))
    }
}
