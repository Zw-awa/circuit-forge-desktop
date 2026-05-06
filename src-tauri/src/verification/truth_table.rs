use serde::{Deserialize, Serialize};
use crate::circuit::types::Signal;

#[derive(Clone, Serialize, Deserialize)]
pub struct TruthTableRow {
    pub inputs: Vec<Signal>,
    pub expected_outputs: Vec<Signal>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum TargetType { SubCircuit, LuaScript }

#[derive(Clone, Serialize, Deserialize)]
pub struct TruthTable {
    pub id: u32,
    pub target_def_id: u32,
    pub target_type: TargetType,
    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
    pub rows: Vec<TruthTableRow>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VerificationFailure {
    pub row_index: usize,
    pub inputs: Vec<Signal>,
    pub expected: Vec<Signal>,
    pub actual: Vec<Signal>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub total_rows: usize,
    pub passed_rows: usize,
    pub failures: Vec<VerificationFailure>,
}
