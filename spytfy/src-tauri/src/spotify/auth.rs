use rspotify::{ClientCredsSpotify, Credentials};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tokio::sync::RwLock;

pub type SpotifyClient = Arc<RwLock<Option<ClientCredsSpotify>>>;

const STORE_FILENAME: &str = "credentials.json";
const KEY_CLIENT_ID: &str = "spotify_client_id";
const KEY_CLIENT_SECRET: &str = "spotify_client_secret";

const DEFAULT_CLIENT_ID: &str = "21485dcf2ab14dce9094c1606105302b";
const DEFAULT_CLIENT_SECRET: &str = "bb15c3a9af9b437b8eaed486acf638fe";

pub fn create_client_state() -> SpotifyClient {
    Arc::new(RwLock::new(None))
}

pub async fn init_from_store(app: &AppHandle, client: &SpotifyClient) {
    let (id, secret) = match load_credentials(app) {
        Ok(Some(creds)) => creds,
        _ => (DEFAULT_CLIENT_ID.to_string(), DEFAULT_CLIENT_SECRET.to_string()),
    };
    if let Ok(spotify) = build_client(&id, &secret).await {
        *client.write().await = Some(spotify);
    }
}

fn load_credentials(app: &AppHandle) -> Result<Option<(String, String)>, String> {
    let store = app.store(STORE_FILENAME).map_err(|e| e.to_string())?;

    let id = store.get(KEY_CLIENT_ID);
    let secret = store.get(KEY_CLIENT_SECRET);

    match (id, secret) {
        (Some(id_val), Some(secret_val)) => {
            let id = id_val.as_str().map(|s| s.to_string());
            let secret = secret_val.as_str().map(|s| s.to_string());
            match (id, secret) {
                (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
                    Ok(Some((id, secret)))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

async fn build_client(client_id: &str, client_secret: &str) -> Result<ClientCredsSpotify, String> {
    let creds = Credentials::new(client_id, client_secret);
    let spotify = ClientCredsSpotify::new(creds);
    spotify
        .request_token()
        .await
        .map_err(|e| format!("Spotify auth failed: {e}"))?;
    Ok(spotify)
}

#[tauri::command]
pub async fn save_spotify_credentials(
    app: AppHandle,
    client: tauri::State<'_, SpotifyClient>,
    client_id: String,
    client_secret: String,
) -> Result<(), String> {
    let spotify = build_client(&client_id, &client_secret).await?;

    let store = app.store(STORE_FILENAME).map_err(|e| e.to_string())?;
    store.set(KEY_CLIENT_ID, serde_json::json!(client_id));
    store.set(KEY_CLIENT_SECRET, serde_json::json!(client_secret));
    store.save().map_err(|e| e.to_string())?;

    *client.write().await = Some(spotify);
    Ok(())
}

#[tauri::command]
pub async fn test_spotify_credentials(
    client: tauri::State<'_, SpotifyClient>,
) -> Result<(), String> {
    let guard = client.read().await;
    match guard.as_ref() {
        Some(spotify) => {
            spotify
                .request_token()
                .await
                .map_err(|e| format!("Auth test failed: {e}"))?;
            Ok(())
        }
        None => Err("No credentials configured".to_string()),
    }
}

#[tauri::command]
pub async fn has_spotify_credentials(
    client: tauri::State<'_, SpotifyClient>,
) -> Result<bool, String> {
    Ok(client.read().await.is_some())
}
