use tauri::State;
use crate::EngineState;
use crate::workshop;
use crate::workshop::types::WorkshopItem;

#[tauri::command]
pub async fn workshop_fetch_index(
    url: Option<String>,
) -> Result<workshop::types::WorkshopIndex, String> {
    workshop::fetch::fetch_index(url.as_deref()).await
}

#[tauri::command]
pub async fn workshop_download_item(
    engine: State<'_, EngineState>,
    file_url: String,
    file_type: String,
) -> Result<serde_json::Value, String> {
    let bytes = workshop::fetch::download_item(&file_url).await?;
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    workshop::import_item(&mut eng, &bytes, &file_type)
}

#[tauri::command]
pub async fn workshop_search(
    query: String,
) -> Result<Vec<WorkshopItem>, String> {
    let index = workshop::fetch::fetch_index(None).await?;
    let lower = query.to_lowercase();
    Ok(index
        .items
        .into_iter()
        .filter(|item| {
            item.name.to_lowercase().contains(&lower)
                || item.description.to_lowercase().contains(&lower)
                || item.tags.iter().any(|t| t.to_lowercase().contains(&lower))
                || item.author.to_lowercase().contains(&lower)
        })
        .collect())
}
