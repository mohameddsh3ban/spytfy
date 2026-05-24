const COMMANDS: &[&str] = &["search_youtube", "download_audio", "cancel_download", "register_in_media_store"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
