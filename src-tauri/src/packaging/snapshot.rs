use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub id: u32,
    pub name: String,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SnapshotIndex {
    pub snapshots: Vec<SnapshotInfo>,
    pub next_id: u32,
}

pub fn create_snapshot(
    name: &str,
    circuit_json: &str,
    snapshots: &mut Vec<(u32, String, String, String)>,
    next_id: &mut u32,
) -> SnapshotInfo {
    let created_at = chrono::Utc::now().to_rfc3339();
    let info = SnapshotInfo {
        id: *next_id,
        name: name.to_string(),
        created_at: created_at.clone(),
    };
    *next_id += 1;
    snapshots.push((info.id, info.name.clone(), created_at, circuit_json.to_string()));
    info
}
