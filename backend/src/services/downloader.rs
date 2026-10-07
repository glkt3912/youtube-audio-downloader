use crate::models::{DownloadItem, DownloadStatus};
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
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
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Own process group, so cancel can also kill yt-dlp's children (ffmpeg).
        #[cfg(unix)]
        cmd.process_group(0);

        let mut child = cmd.spawn().context("Failed to spawn yt-dlp process")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;
        // Read stderr concurrently so a full pipe can't block yt-dlp; keep the last ERROR line.
        let last_error = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buf = Vec::new();
            let mut last_error = None;
            // Read to EOF even on bad bytes; stopping early would EPIPE yt-dlp.
            while reader.read_until(b'\n', &mut buf).await.unwrap_or(0) > 0 {
                let line = String::from_utf8_lossy(&buf);
                let line = line.trim_end();
                eprintln!("{line}");
                if let Some(msg) = line.strip_prefix("ERROR: ") {
                    last_error = Some(msg.to_string());
                }
                buf.clear();
            }
            last_error
        });

        let run = async {
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                // Lossy decode: a non-UTF-8 line must not abort the download.
                let n = reader
                    .read_until(b'\n', &mut buf)
                    .await
                    .context("Failed to read yt-dlp output")?;
                if n == 0 {
                    break;
                }
                let line = String::from_utf8_lossy(&buf);

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
            child
                .wait()
                .await
                .context("Failed to wait for yt-dlp process")
        };

        // Cancel is honoured both while reading output and while waiting for exit.
        let status = tokio::select! {
            _ = cancel.notified() => None,
            status = run => Some(status?),
        };
        let Some(status) = status else {
            // ponytail: leaves yt-dlp's .part file behind; delete it here if that matters
            kill_tree(&mut child).await;
            return Ok(());
        };

        if status.success() {
            let mut locked = item.lock().await;
            locked.update_status(DownloadStatus::Completed);
            locked.update_progress(100.0);
            Ok(())
        } else {
            let error_msg =
                last_error.await.ok().flatten().unwrap_or_else(|| {
                    format!("yt-dlp failed with exit code: {:?}", status.code())
                });
            item.lock().await.set_error(error_msg.clone());
            Err(anyhow::anyhow!(error_msg))
        }
    }
}

/// Kill yt-dlp and, on Unix, everything in its process group (ffmpeg).
/// ponytail: on Windows only yt-dlp.exe is killed; a PyInstaller onefile build's
/// inner process and ffmpeg may survive. Use a Job Object if that matters.
async fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // SAFETY: plain syscall; pid is our child's group id from process_group(0).
        unsafe { libc::killpg(pid as libc::pid_t, libc::SIGKILL) };
    }
    // Reaps the child; errors only if it already exited, which is fine here.
    let _ = child.kill().await;
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
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::time::Duration;

    /// Install a fake yt-dlp on PATH once (tests run in parallel; PATH is process-wide).
    /// URLs containing "fail" exit with a 403 error; others spawn a grandchild
    /// (like ffmpeg), write its pid to <dir>/<last url segment>.pid, and hang.
    fn fake_dir() -> &'static PathBuf {
        static DIR: OnceLock<PathBuf> = OnceLock::new();
        DIR.get_or_init(|| {
            let dir = std::env::temp_dir().join(format!("fake-ytdlp-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let bin = dir.join("yt-dlp");
            let script = format!(
                r#"#!/bin/sh
for a in "$@"; do url="$a"; done
case "$url" in
  *fail*) echo 'WARNING: old' >&2
          echo 'ERROR: unable to download video data: HTTP Error 403: Forbidden' >&2
          exit 1 ;;
  *) echo '[download]  10.0%'
     sleep 30 &
     echo $! > '{}/'"${{url##*/}}".pid
     wait ;;
esac
"#,
                dir.display()
            );
            std::fs::write(&bin, script).unwrap();
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
            let path = std::env::var("PATH").unwrap_or_default();
            std::env::set_var("PATH", format!("{}:{}", dir.display(), path));
            dir
        })
    }

    fn new_item(url: &str) -> Arc<Mutex<DownloadItem>> {
        Arc::new(Mutex::new(DownloadItem::new(
            url.into(),
            AudioFormat::Mp3,
            Quality::Best,
        )))
    }

    fn alive(pid: i32) -> bool {
        // SAFETY: signal 0 only checks for existence.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    #[tokio::test]
    async fn cancel_kills_process_tree() {
        let dir = fake_dir();
        let item = new_item("https://youtu.be/hang");
        let cancel = item.lock().await.cancel.clone();
        let task = tokio::spawn({
            let item = item.clone();
            async move { Downloader::new().download(item).await }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        let grandchild: i32 = std::fs::read_to_string(dir.join("hang.pid"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(alive(grandchild));
        cancel.notify_one();

        let result = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("download did not stop after cancel");
        assert!(result.unwrap().is_ok());
        assert_ne!(item.lock().await.status, DownloadStatus::Completed);

        // Orphaned grandchild is reaped by init asynchronously; give it a moment.
        for _ in 0..20 {
            if !alive(grandchild) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!alive(grandchild), "grandchild survived cancel");
    }

    #[tokio::test]
    async fn failure_reports_ytdlp_error_line() {
        fake_dir();
        let item = new_item("https://youtu.be/fail");

        assert!(Downloader::new().download(item.clone()).await.is_err());
        assert_eq!(
            item.lock().await.error.as_deref(),
            Some("unable to download video data: HTTP Error 403: Forbidden")
        );
    }
}
