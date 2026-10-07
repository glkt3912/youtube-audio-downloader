#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod models;
mod services;
mod utils;

use commands::{add_download, cancel_download, check_deps, get_install_guide, get_queue};
use services::DownloadQueue;
use std::sync::Arc;

fn main() {
    // Apps launched from Finder get a minimal PATH without Homebrew, so yt-dlp/ffmpeg aren't found.
    #[cfg(target_os = "macos")]
    {
        // Fall back to the libc default; an empty element would mean "search cwd".
        let path = std::env::var("PATH")
            .ok()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "/usr/bin:/bin".into());
        // Runs before any thread (tokio/tauri) is spawned, so set_var is sound.
        std::env::set_var("PATH", format!("{path}:/opt/homebrew/bin:/usr/local/bin"));
    }

    let queue = Arc::new(DownloadQueue::new(3));
    queue.start_processing();

    tauri::Builder::default()
        .manage(queue)
        .invoke_handler(tauri::generate_handler![
            add_download,
            get_queue,
            cancel_download,
            check_deps,
            get_install_guide,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
