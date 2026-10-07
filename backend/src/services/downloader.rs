use crate::models::{DownloadItem, DownloadStatus};
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
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
        let (url, format, quality, cancel) = {
            let item_lock = item.lock().await;
            (
                item_lock.url.clone(),
                item_lock.format,
                item_lock.quality,
                item_lock.cancel.clone(),
            )
        };

        item.lock().await.update_status(DownloadStatus::Downloading);

        let output_template = format!("{}/%(title)s.%(ext)s", self.download_dir);

        let mut cmd = Command::new("yt-dlp");
        // ponytail: YouTube 403s https audio streams without a PO token; HLS works. Drop when yt-dlp handles it.
        cmd.arg("-f")
            .arg("ba[protocol^=m3u8]/ba")
            .arg("--extract-audio")
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
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let mut child = cmd.spawn().context("Failed to spawn yt-dlp process")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;

        let mut lines = BufReader::new(stdout).lines();

        loop {
            let line = tokio::select! {
                _ = cancel.notified() => {
                    // ponytail: leaves yt-dlp's .part file behind; delete it here if that matters
                    child.kill().await.context("Failed to kill yt-dlp process")?;
                    return Ok(());
                }
                line = lines.next_line() => line.context("Failed to read yt-dlp output")?,
            };
            let Some(line) = line else { break };

            let mut locked = item.lock().await;
            if let Some(title) = TITLE_REGEX.captures(&line).and_then(|c| c.get(1)) {
                locked.set_title(title.as_str().to_string());
            }
            if let Some(progress) = PROGRESS_REGEX
                .captures(&line)
                .and_then(|c| c.get(1))
                .and_then(|p| p.as_str().parse::<f32>().ok())
            {
                locked.update_progress(progress);
            }
            if line.contains("[ExtractAudio]") || line.contains("Merging formats") {
                locked.update_status(DownloadStatus::Converting);
            }
        }

        let status = child
            .wait()
            .await
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::models::{AudioFormat, Quality};
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    #[tokio::test]
    async fn cancel_kills_running_process() {
        // Fake yt-dlp that prints progress once and then hangs.
        let dir = std::env::temp_dir().join(format!("fake-ytdlp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("yt-dlp");
        std::fs::write(&bin, "#!/bin/sh\necho '[download]  10.0%'\nsleep 30\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", dir.display(), path));

        let item = Arc::new(Mutex::new(DownloadItem::new(
            "https://youtu.be/x".into(),
            AudioFormat::Mp3,
            Quality::Best,
        )));
        let cancel = item.lock().await.cancel.clone();
        let task = tokio::spawn({
            let item = item.clone();
            async move { Downloader::new().download(item).await }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        cancel.notify_one();

        let result = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("download did not stop after cancel");
        assert!(result.unwrap().is_ok());
        assert_ne!(item.lock().await.status, DownloadStatus::Completed);
        std::fs::remove_dir_all(&dir).ok();
    }
}
