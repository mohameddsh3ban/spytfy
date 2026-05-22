use std::path::PathBuf;
use tauri::Manager;

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| {
        #[cfg(not(target_os = "android"))]
        {
            dirs::data_dir().unwrap_or_else(|| std::env::current_dir().unwrap())
        }
        #[cfg(target_os = "android")]
        {
            std::env::current_dir().unwrap()
        }
    })
}

pub fn default_output_dir(app: &tauri::AppHandle) -> PathBuf {
    #[cfg(not(target_os = "android"))]
    {
        dirs::audio_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
    }
    #[cfg(target_os = "android")]
    {
        app.path().app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap())
            .join("Music")
    }
}

pub fn default_concurrency() -> u8 {
    #[cfg(not(target_os = "android"))]
    { 3 }
    #[cfg(target_os = "android")]
    { 1 }
}
