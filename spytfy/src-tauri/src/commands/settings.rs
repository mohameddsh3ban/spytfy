use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub output_root: String,
    pub concurrency: u8,
    pub bitrate_kbps: u16,
    pub overwrite_existing: bool,
    pub write_cover_jpg: bool,
    pub naming_template: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub output_root: Option<String>,
    pub concurrency: Option<u8>,
    pub bitrate_kbps: Option<u16>,
    pub overwrite_existing: Option<bool>,
    pub write_cover_jpg: Option<bool>,
    pub naming_template: Option<String>,
}

impl Settings {
    fn defaults() -> Self {
        let music_dir = {
            #[cfg(not(target_os = "android"))]
            { dirs::audio_dir().or_else(dirs::home_dir).unwrap_or_default() }
            #[cfg(target_os = "android")]
            { std::path::PathBuf::from("/storage/emulated/0/Music") }
        }.to_string_lossy().to_string();

        Self {
            output_root: music_dir,
            concurrency: crate::platform::default_concurrency(),
            bitrate_kbps: 320,
            overwrite_existing: false,
            write_cover_jpg: true,
            naming_template: "{folder}/{number} - {artist} - {title}".to_string(),
        }
    }
}

async fn get_setting(pool: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

async fn set_setting(pool: &SqlitePool, key: &str, value: &str) {
    let _ = sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')"
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await;
}

async fn load_settings(pool: &SqlitePool) -> Settings {
    let defaults = Settings::defaults();

    Settings {
        output_root: get_setting(pool, "output_root")
            .await
            .unwrap_or(defaults.output_root),
        concurrency: get_setting(pool, "concurrency")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.concurrency),
        bitrate_kbps: get_setting(pool, "bitrate_kbps")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.bitrate_kbps),
        overwrite_existing: get_setting(pool, "overwrite_existing")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.overwrite_existing),
        write_cover_jpg: get_setting(pool, "write_cover_jpg")
            .await
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.write_cover_jpg),
        naming_template: get_setting(pool, "naming_template")
            .await
            .unwrap_or(defaults.naming_template),
    }
}

#[tauri::command]
pub async fn get_settings(pool: State<'_, SqlitePool>) -> Result<Settings, String> {
    Ok(load_settings(&pool).await)
}

#[tauri::command]
pub async fn update_settings(
    pool: State<'_, SqlitePool>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let pool_ref: &SqlitePool = &pool;

    if let Some(v) = &patch.output_root {
        set_setting(pool_ref, "output_root", v).await;
    }
    if let Some(v) = patch.concurrency {
        set_setting(pool_ref, "concurrency", &v.to_string()).await;
    }
    if let Some(v) = patch.bitrate_kbps {
        set_setting(pool_ref, "bitrate_kbps", &v.to_string()).await;
    }
    if let Some(v) = patch.overwrite_existing {
        set_setting(pool_ref, "overwrite_existing", &v.to_string()).await;
    }
    if let Some(v) = patch.write_cover_jpg {
        set_setting(pool_ref, "write_cover_jpg", &v.to_string()).await;
    }
    if let Some(v) = &patch.naming_template {
        set_setting(pool_ref, "naming_template", v).await;
    }

    Ok(load_settings(pool_ref).await)
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    Ok(())
}
