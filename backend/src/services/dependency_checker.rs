use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub yt_dlp_installed: bool,
    pub ffmpeg_installed: bool,
    pub all_installed: bool,
}

impl DependencyStatus {
    pub fn check() -> Self {
        let yt_dlp_installed = check_yt_dlp();
        let ffmpeg_installed = check_ffmpeg();
        let all_installed = yt_dlp_installed && ffmpeg_installed;

        Self {
            yt_dlp_installed,
            ffmpeg_installed,
            all_installed,
        }
    }
}

fn check_yt_dlp() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn check_ffmpeg() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallGuide {
    pub platform: String,
    pub yt_dlp_command: String,
    pub ffmpeg_command: String,
    pub notes: Vec<String>,
}

impl InstallGuide {
    pub fn get() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                platform: "macOS".to_string(),
                yt_dlp_command: "brew install yt-dlp".to_string(),
                ffmpeg_command: "brew install ffmpeg".to_string(),
                notes: vec![
                    "Homebrew must be installed first".to_string(),
                    "Visit https://brew.sh for Homebrew installation".to_string(),
                ],
            }
        }

        #[cfg(target_os = "windows")]
        {
            Self {
                platform: "Windows".to_string(),
                yt_dlp_command: "winget install yt-dlp.yt-dlp".to_string(),
                ffmpeg_command: "winget install Gyan.FFmpeg".to_string(),
                notes: vec![
                    "Windows 10+ required for winget".to_string(),
                    "Alternatively, download binaries from official websites".to_string(),
                ],
            }
        }

        #[cfg(target_os = "linux")]
        {
            Self {
                platform: "Linux".to_string(),
                yt_dlp_command: "sudo apt install yt-dlp".to_string(),
                ffmpeg_command: "sudo apt install ffmpeg".to_string(),
                notes: vec![
                    "For Debian/Ubuntu based systems".to_string(),
                    "For other distros, use your package manager".to_string(),
                ],
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Self {
                platform: "Unknown".to_string(),
                yt_dlp_command: "Visit https://github.com/yt-dlp/yt-dlp".to_string(),
                ffmpeg_command: "Visit https://ffmpeg.org/download.html".to_string(),
                notes: vec!["Please install manually".to_string()],
            }
        }
    }
}
