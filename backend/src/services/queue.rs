use crate::models::{AudioFormat, DownloadItem, DownloadStatus, Quality};
use crate::services::downloader::Downloader;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use tauri::async_runtime::JoinHandle;

pub struct DownloadQueue {
    queue: Arc<Mutex<VecDeque<Arc<tokio::sync::Mutex<DownloadItem>>>>>,
    active: Arc<Mutex<Vec<Arc<tokio::sync::Mutex<DownloadItem>>>>>,
    all_items: Arc<Mutex<Vec<Arc<tokio::sync::Mutex<DownloadItem>>>>>,
    max_concurrent: usize,
    downloader: Arc<Downloader>,
    task_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl DownloadQueue {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active: Arc::new(Mutex::new(Vec::new())),
            all_items: Arc::new(Mutex::new(Vec::new())),
            max_concurrent,
            downloader: Arc::new(Downloader::new()),
            task_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_item(&self, url: String, format: AudioFormat, quality: Quality) -> String {
        let item = DownloadItem::new(url, format, quality);
        let id = item.id.clone();
        let item_arc = Arc::new(tokio::sync::Mutex::new(item));

        self.queue.lock().push_back(item_arc.clone());
        self.all_items.lock().push(item_arc);

        id
    }

    pub async fn get_all_items(&self) -> Vec<DownloadItem> {
        let all_items = self.all_items.lock().clone();
        let mut items = Vec::new();

        for item_arc in all_items {
            let item = item_arc.lock().await.clone();
            items.push(item);
        }

        items
    }

    pub async fn remove_item(&self, id: &str) -> bool {
        let mut all_items = self.all_items.lock();

        if let Some(pos) = all_items.iter().position(|item_arc| {
            let item = item_arc.try_lock();
            if let Ok(item) = item {
                item.id == id
            } else {
                false
            }
        }) {
            let item_arc = all_items.remove(pos);

            if let Ok(mut item) = item_arc.try_lock() {
                item.update_status(DownloadStatus::Cancelled);
            }

            let mut queue = self.queue.lock();
            queue.retain(|item_arc| {
                let item = item_arc.try_lock();
                if let Ok(item) = item {
                    item.id != id
                } else {
                    true
                }
            });

            true
        } else {
            false
        }
    }

    pub fn start_processing(&self) {
        let queue = self.queue.clone();
        let active = self.active.clone();
        let downloader = self.downloader.clone();
        let max_concurrent = self.max_concurrent;
        let task_handles = self.task_handles.clone();

        tauri::async_runtime::spawn(async move {
            loop {
                let should_start = {
                    let active_lock = active.lock();
                    let queue_lock = queue.lock();
                    active_lock.len() < max_concurrent && !queue_lock.is_empty()
                };

                if should_start {
                    let item_arc = {
                        let mut queue_lock = queue.lock();
                        queue_lock.pop_front()
                    };

                    if let Some(item_arc) = item_arc {
                        active.lock().push(item_arc.clone());

                        let active_clone = active.clone();
                        let downloader_clone = downloader.clone();
                        let item_clone = item_arc.clone();

                        let handle = tauri::async_runtime::spawn(async move {
                            let result = downloader_clone.download(item_clone.clone()).await;

                            if let Err(e) = result {
                                let mut item = item_clone.lock().await;
                                item.set_error(e.to_string());
                            }

                            let mut active_lock = active_clone.lock();
                            active_lock
                                .retain(|active_item| !Arc::ptr_eq(active_item, &item_clone));
                        });

                        task_handles.lock().push(handle);
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
    }
}

impl Default for DownloadQueue {
    fn default() -> Self {
        Self::new(3)
    }
}
