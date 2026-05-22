const COMMANDS: &[&str] = &["search_youtube", "download_audio", "cancel_download"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
