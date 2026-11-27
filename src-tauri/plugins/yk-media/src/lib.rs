use serde::Serialize;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

#[derive(Serialize, Clone, Default, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub title: String,
    pub artist: String,
    pub cover: Option<String>,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub active: bool,
}

pub struct Media<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: tauri::plugin::PluginHandle<R>,
    _r: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> Media<R> {
    #[allow(unused_variables)]
    pub fn update(&self, s: &MediaState) {
        #[cfg(target_os = "android")]
        let _ = self.handle.run_mobile_plugin::<serde_json::Value>("update", s);
    }

    pub fn outputs(&self) {
        #[cfg(target_os = "android")]
        let _ = self.handle.run_mobile_plugin::<serde_json::Value>("outputs", serde_json::json!({}));
    }

    #[allow(unused_variables)]
    pub fn listen<F: Fn(serde_json::Value) + Send + Sync + 'static>(&self, f: F) {
        #[cfg(target_os = "android")]
        {
            use tauri::ipc::{Channel, InvokeResponseBody};
            let channel: Channel<serde_json::Value> = Channel::new(move |body| {
                if let InvokeResponseBody::Json(s) = body {
                    if let Ok(v) = serde_json::from_str(&s) {
                        f(v);
                    }
                }
                Ok(())
            });
            let _ = self.handle.run_mobile_plugin::<serde_json::Value>("listen", serde_json::json!({ "channel": channel }));
        }
    }
}

pub trait MediaExt<R: Runtime> {
    fn media(&self) -> &Media<R>;
}

impl<R: Runtime, T: Manager<R>> MediaExt<R> for T {
    fn media(&self) -> &Media<R> {
        self.state::<Media<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("yk-media")
        .setup(|app, _api| {
            #[cfg(target_os = "android")]
            let handle = _api.register_android_plugin("dev.yokyznt.yokisfy.media", "MediaPlugin")?;
            app.manage(Media::<R> {
                #[cfg(target_os = "android")]
                handle,
                _r: std::marker::PhantomData,
            });
            Ok(())
        })
        .build()
}
