use crate::models::{AudioFormat, DownloadItem, DownloadStatus, Quality};
use crate::services::downloader::Downloader;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

pub struct DownloadQueue {
    queue: Arc<Mutex<VecDeque<Arc<tokio::sync::Mutex<DownloadItem>>>>>,
    active: Arc<Mutex<Vec<Arc<tokio::sync::Mutex<DownloadItem>>>>>,
    all_items: Arc<Mutex<Vec<Arc<tokio::sync::Mutex<DownloadItem>>>>>,
    max_concurrent: usize,
    downloader: Arc<Downloader>,
}

impl DownloadQueue {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active: Arc::new(Mutex::new(Vec::new())),
            all_items: Arc::new(Mutex::new(Vec::new())),
            max_concurrent,
            downloader: Arc::new(Downloader::new()),
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
        // Snapshot first: never hold the parking_lot guard across .await.
        let items = self.all_items.lock().clone();

        // Wait for each item's lock instead of try_lock, so a cancel can't be dropped
        // just because the downloader happens to be updating progress.
        let mut target = None;
        for item_arc in items {
            let mut item = item_arc.lock().await;
            if item.id == id {
                item.update_status(DownloadStatus::Cancelled);
                item.cancel.notify_one();
                drop(item);
                target = Some(item_arc);
                break;
            }
        }
        let Some(target) = target else {
            return false;
        };

        self.all_items
            .lock()
            .retain(|item_arc| !Arc::ptr_eq(item_arc, &target));
        self.queue
            .lock()
            .retain(|item_arc| !Arc::ptr_eq(item_arc, &target));
        true
    }

    /// Drop completed/failed/cancelled items; queued and in-progress items stay.
    pub fn clear_finished(&self) {
        self.all_items
            .lock()
            .retain(|item_arc| match item_arc.try_lock() {
                Ok(item) => !matches!(
                    item.status,
                    DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
                ),
                // Briefly held elsewhere (downloader or get_all_items); skip it this time.
                Err(_) => true,
            });
    }

    pub fn start_processing(&self) {
        let queue = self.queue.clone();
        let active = self.active.clone();
        let downloader = self.downloader.clone();
        let max_concurrent = self.max_concurrent;

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

                        tokio::spawn(async move {
                            let result = downloader_clone.download(item_clone.clone()).await;

                            if let Err(e) = result {
                                let mut item = item_clone.lock().await;
                                item.set_error(e.to_string());
                            }

                            let mut active_lock = active_clone.lock();
                            active_lock
                                .retain(|active_item| !Arc::ptr_eq(active_item, &item_clone));
                        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn clear_finished_keeps_only_unfinished() {
        let queue = DownloadQueue::new(1);
        for _ in 0..3 {
            queue.add_item("https://youtu.be/x".into(), AudioFormat::Mp3, Quality::Best);
        }
        {
            let items = queue.all_items.lock().clone();
            items[0]
                .lock()
                .await
                .update_status(DownloadStatus::Completed);
            items[1].lock().await.set_error("boom".into());
        }

        queue.clear_finished();

        let left = queue.get_all_items().await;
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].status, DownloadStatus::Queued);
    }

    #[tokio::test]
    async fn clear_finished_skips_locked_items() {
        let queue = DownloadQueue::new(1);
        queue.add_item("https://youtu.be/x".into(), AudioFormat::Mp3, Quality::Best);
        let item = queue.all_items.lock()[0].clone();
        let mut guard = item.lock().await;
        guard.update_status(DownloadStatus::Completed);

        queue.clear_finished();
        assert_eq!(queue.all_items.lock().len(), 1);

        drop(guard);
        queue.clear_finished();
        assert!(queue.all_items.lock().is_empty());
    }

    #[tokio::test]
    async fn remove_item_notifies_even_while_item_is_locked() {
        let queue = Arc::new(DownloadQueue::new(1));
        let id = queue.add_item("https://youtu.be/x".into(), AudioFormat::Mp3, Quality::Best);
        let item = queue.all_items.lock()[0].clone();
        let cancel = item.lock().await.cancel.clone();

        // Simulate the downloader holding the item lock when cancel arrives.
        let guard = item.lock().await;
        let remove = tokio::spawn({
            let queue = queue.clone();
            async move { queue.remove_item(&id).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(guard);

        assert!(remove.await.unwrap());
        tokio::time::timeout(Duration::from_secs(1), cancel.notified())
            .await
            .expect("cancel was not notified");
        assert!(queue.all_items.lock().is_empty());
        assert!(queue.queue.lock().is_empty());
    }
}
