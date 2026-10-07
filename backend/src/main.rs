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
        let path = std::env::var("PATH").unwrap_or_default();
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
