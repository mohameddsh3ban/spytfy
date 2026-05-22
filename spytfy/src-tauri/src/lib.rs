mod commands;
mod db;
mod download;
#[cfg(not(target_os = "android"))]
mod ocr;
mod queue;
mod spotify;

use commands::settings;
use download::pipeline;
#[cfg(not(target_os = "android"))]
use ocr::commands as ocr_cmds;
use queue::commands as queue_cmds;
use spotify::{auth, resolver, scraper};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let spotify_client = auth::create_client_state();

            tauri::async_runtime::block_on(async {
                let app_data_dir = app_handle.path().app_data_dir()
                    .unwrap_or_else(|_| dirs::data_dir().unwrap_or_else(|| {
                        std::env::current_dir().expect("failed to get current dir")
                    }));

                let pool = db::init_pool(app_data_dir)
                    .await
                    .expect("failed to initialize database");

                // Create channel for worker pool
                let (tx, rx) = tokio::sync::mpsc::channel(50);

                let queue_mgr = queue::manager::QueueManager::new(pool.clone(), tx);

                // Merge any duplicate batches from prior runs
                queue_mgr.merge_duplicate_batches().await;
                // Full reset on startup — includes 'pending' since channel is empty
                queue_mgr.reset_all_stuck_jobs().await;

                // Spawn worker pool
                let worker_pool = queue::worker::WorkerPool::spawn(
                    rx,
                    queue_mgr.clone(),
                    app_handle.clone(),
                    2,
                );

                app_handle.manage(pool);
                app_handle.manage(queue_mgr.clone());
                app_handle.manage(worker_pool);

                // Push queued jobs in background — don't block startup
                tokio::spawn(async move {
                    queue_mgr.push_all_queued_jobs().await;
                });

                auth::init_from_store(&app_handle, &spotify_client).await;
                app_handle.manage(spotify_client);
            });

            Ok(())
        })
        .invoke_handler({
            #[cfg(not(target_os = "android"))]
            {
                tauri::generate_handler![
                    settings::get_settings,
                    settings::update_settings,
                    settings::open_folder,
                    auth::save_spotify_credentials,
                    auth::test_spotify_credentials,
                    auth::has_spotify_credentials,
                    resolver::resolve_url,
                    scraper::debug_scrape,
                    scraper::resolve_from_json,
                    pipeline::download_track,
                    queue_cmds::enqueue_download,
                    queue_cmds::list_batches,
                    queue_cmds::list_jobs,
                    queue_cmds::pause_batch,
                    queue_cmds::resume_batch,
                    queue_cmds::cancel_batch,
                    queue_cmds::retry_job,
                    queue_cmds::resume_queued,
                    queue_cmds::retry_all_failed,
                    queue_cmds::pick_candidate,
                    queue_cmds::list_failed_jobs,
                    ocr_cmds::process_screenshots,
                    ocr_cmds::debug_ocr,
                    ocr_cmds::create_playlist_from_tracks,
                    ocr_cmds::parse_text_tracklist,
                    ocr_cmds::parse_spotify_html,
                    ocr_cmds::scrape_playlist_tracks,
                ]
            }
            #[cfg(target_os = "android")]
            {
                tauri::generate_handler![
                    settings::get_settings,
                    settings::update_settings,
                    settings::open_folder,
                    auth::save_spotify_credentials,
                    auth::test_spotify_credentials,
                    auth::has_spotify_credentials,
                    resolver::resolve_url,
                    scraper::debug_scrape,
                    scraper::resolve_from_json,
                    pipeline::download_track,
                    queue_cmds::enqueue_download,
                    queue_cmds::list_batches,
                    queue_cmds::list_jobs,
                    queue_cmds::pause_batch,
                    queue_cmds::resume_batch,
                    queue_cmds::cancel_batch,
                    queue_cmds::retry_job,
                    queue_cmds::resume_queued,
                    queue_cmds::retry_all_failed,
                    queue_cmds::pick_candidate,
                    queue_cmds::list_failed_jobs,
                ]
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Spytfy");
}
