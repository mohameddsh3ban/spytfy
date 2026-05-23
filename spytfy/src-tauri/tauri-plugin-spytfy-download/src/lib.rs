use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.tauri.spytfy_download";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("spytfy-download")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            api.register_android_plugin(PLUGIN_IDENTIFIER, "DownloadPlugin")?;
            Ok(())
        })
        .build()
}
