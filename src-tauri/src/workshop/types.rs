use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopIndex {
    pub version: u32,
    #[serde(rename = "indexUrl")]
    pub index_url: Option<String>,
    pub items: Vec<WorkshopItem>,
    pub tutorials: Vec<TutorialLink>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub download_url: String,
    pub thumbnail_url: Option<String>,
    pub tags: Vec<String>,
    pub version: String,
    pub updated_at: String,
    pub file_size: u64,
    pub file_type: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorialLink {
    pub title: String,
    pub url: String,
    pub language: String,
    pub tags: Vec<String>,
}
