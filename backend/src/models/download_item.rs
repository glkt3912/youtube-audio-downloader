use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub format: AudioFormat,
    pub quality: Quality,
    pub status: DownloadStatus,
    pub progress: f32,
    pub error: Option<String>,
}

impl DownloadItem {
    pub fn new(url: String, format: AudioFormat, quality: Quality) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            title: None,
            format,
            quality,
            status: DownloadStatus::Queued,
            progress: 0.0,
            error: None,
        }
    }

    pub fn update_status(&mut self, status: DownloadStatus) {
        self.status = status;
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress;
    }

    pub fn set_title(&mut self, title: String) {
        self.title = Some(title);
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.status = DownloadStatus::Failed;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AudioFormat {
    Mp3,
    M4a,
    Opus,
    Aac,
    Flac,
    Wav,
}

impl AudioFormat {
    pub fn as_str(&self) -> &str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Opus => "opus",
            AudioFormat::Aac => "aac",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Quality {
    Best,
    High,
    Medium,
    Low,
}

impl Quality {
    pub fn as_str(&self) -> &str {
        match self {
            Quality::Best => "0", // yt-dlp uses 0 for best quality
            Quality::High => "192K",
            Quality::Medium => "128K",
            Quality::Low => "96K",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Converting,
    Completed,
    Failed,
    Cancelled,
}
