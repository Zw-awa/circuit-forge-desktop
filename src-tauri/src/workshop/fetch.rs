use super::types::WorkshopIndex;

const DEFAULT_INDEX_URL: &str =
    "https://zw-awa.github.io/circuit-forge-workshop/index.json";

pub async fn fetch_index(url: Option<&str>) -> Result<WorkshopIndex, String> {
    let target = url.unwrap_or(DEFAULT_INDEX_URL);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(target).send().await.map_err(|e| e.to_string())?;
    resp.json::<WorkshopIndex>().await.map_err(|e| e.to_string())
}

pub async fn download_item(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}
