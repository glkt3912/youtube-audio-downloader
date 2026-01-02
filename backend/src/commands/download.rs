use crate::models::{AudioFormat, DownloadItem};
use crate::services::DownloadQueue;
use crate::utils::validate_urls;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn add_download(
    urls: String,
    format: AudioFormat,
    quality: crate::models::Quality,
    queue: State<'_, Arc<DownloadQueue>>,
) -> Result<Vec<String>, String> {
    let url_list: Vec<String> = urls
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let valid_urls = validate_urls(url_list)?;

    let mut ids = Vec::new();
    for url in valid_urls {
        let id = queue.add_item(url, format, quality);
        ids.push(id);
    }

    Ok(ids)
}

#[tauri::command]
pub async fn get_queue(queue: State<'_, Arc<DownloadQueue>>) -> Result<Vec<DownloadItem>, String> {
    Ok(queue.get_all_items().await)
}

#[tauri::command]
pub async fn cancel_download(
    id: String,
    queue: State<'_, Arc<DownloadQueue>>,
) -> Result<bool, String> {
    Ok(queue.remove_item(&id).await)
}
