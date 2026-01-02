use crate::services::{DependencyStatus, InstallGuide};

#[tauri::command]
pub fn check_deps() -> DependencyStatus {
    DependencyStatus::check()
}

#[tauri::command]
pub fn get_install_guide() -> InstallGuide {
    InstallGuide::get()
}
