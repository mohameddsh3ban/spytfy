use serde::de::DeserializeOwned;
use tauri::{
    plugin::{Builder, PluginApi, PluginHandle, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "app.tauri.spytfy_download";

pub struct SpytfyDownload<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> SpytfyDownload<R> {
    pub fn run_mobile_plugin<T: DeserializeOwned>(
        &self,
        command: &str,
        payload: impl serde::Serialize,
    ) -> Result<T, String> {
        self.0
            .run_mobile_plugin(command, payload)
            .map_err(|e| format!("Plugin call failed: {e}"))
    }
}

fn init_plugin<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<SpytfyDownload<R>, Box<dyn std::error::Error>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "DownloadPlugin")?;
    #[cfg(not(target_os = "android"))]
    let handle = api.register_android_plugin("", "")?;
    Ok(SpytfyDownload(handle))
}

pub trait SpytfyDownloadExt<R: Runtime> {
    fn spytfy_download(&self) -> &SpytfyDownload<R>;
}

impl<R: Runtime, T: tauri::Manager<R>> SpytfyDownloadExt<R> for T {
    fn spytfy_download(&self) -> &SpytfyDownload<R> {
        self.state::<SpytfyDownload<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R, ()>::new("spytfy-download")
        .setup(|app, api| {
            let handle = init_plugin(app, api)?;
            app.manage(handle);
            Ok(())
        })
        .build()
}
