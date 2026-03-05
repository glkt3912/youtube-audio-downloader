use crate::models::{DownloadItem, DownloadStatus};
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;

static PROGRESS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[download\]\s+(\d+\.?\d*)%").unwrap());
static TITLE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[info\]\s+(.+?):\s+Downloading").unwrap());

pub struct Downloader {
    download_dir: String,
}

impl Downloader {
    pub fn new() -> Self {
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .to_string_lossy()
            .to_string();

        Self { download_dir }
    }

    pub async fn download(&self, item: Arc<Mutex<DownloadItem>>) -> Result<()> {
        let (url, format, quality) = {
            let item_lock = item.lock().await;
            (item_lock.url.clone(), item_lock.format, item_lock.quality)
        };

        item.lock().await.update_status(DownloadStatus::Downloading);

        let output_template = format!("{}/%(title)s.%(ext)s", self.download_dir);

        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--extract-audio")
            .arg("--audio-format")
            .arg(format.as_str())
            .arg("--audio-quality")
            .arg(quality.as_str())
            .arg("--embed-thumbnail")
            .arg("--add-metadata")
            .arg("--output")
            .arg(&output_template)
            .arg("--newline")
            .arg("--no-playlist")
            .arg(&url)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = cmd.spawn().context("Failed to spawn yt-dlp process")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;

        let item_clone = item.clone();
        tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(captures) = TITLE_REGEX.captures(&line) {
                    if let Some(title) = captures.get(1) {
                        item_clone
                            .blocking_lock()
                            .set_title(title.as_str().to_string());
                    }
                }
                if let Some(captures) = PROGRESS_REGEX.captures(&line) {
                    if let Some(progress_str) = captures.get(1) {
                        if let Ok(progress) = progress_str.as_str().parse::<f32>() {
                            item_clone.blocking_lock().update_progress(progress);
                        }
                    }
                }
                if line.contains("[ExtractAudio]") || line.contains("Merging formats") {
                    item_clone
                        .blocking_lock()
                        .update_status(DownloadStatus::Converting);
                }
            }
        })
        .await
        .context("stdout reading task panicked")?;

        let status = tokio::task::spawn_blocking(move || child.wait())
            .await
            .context("process wait task panicked")?
            .context("Failed to wait for yt-dlp process")?;

        if status.success() {
            let mut locked = item.lock().await;
            locked.update_status(DownloadStatus::Completed);
            locked.update_progress(100.0);
            Ok(())
        } else {
            let error_msg = format!("yt-dlp failed with exit code: {:?}", status.code());
            item.lock().await.set_error(error_msg.clone());
            Err(anyhow::anyhow!(error_msg))
        }
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}
