use super::sandbox::LuaSandbox;

pub fn validate_script(source: &str, input_count: usize, output_count: usize) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let sandbox = match LuaSandbox::new() {
        Ok(s) => s,
        Err(e) => { errors.push(e); return Err(errors); }
    };

    match sandbox.evaluate(
        source,
        &vec![crate::circuit::types::Signal::Low; input_count],
        &serde_json::json!({}),
        false,
    ) {
        Ok((outputs, _)) => {
            if outputs.len() != output_count {
                errors.push(format!(
                    "evaluate returned {} outputs, expected {}",
                    outputs.len(), output_count
                ));
            }
        }
        Err(e) => {
            errors.push(format!("Runtime error: {}", e));
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
