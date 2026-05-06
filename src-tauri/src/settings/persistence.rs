use crate::settings::{LlmProvider, ProviderSettings};
use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const SETTINGS_PATH: &str = "settings.json";

pub fn save_settings(app: &AppHandle, settings: &ProviderSettings) -> Result<(), String> {
    let store = app.store(SETTINGS_PATH).map_err(|e| e.to_string())?;

    store.set("provider", json!(settings.provider));
    store.set("cloudEnabled", json!(settings.cloud_enabled));
    store.set("localModelPath", json!(settings.local_model_path));

    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

pub fn load_settings(app: &AppHandle) -> ProviderSettings {
    let store = match app.store(SETTINGS_PATH) {
        Ok(s) => s,
        Err(_) => return default_settings(),
    };

    let provider = store
        .get("provider")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(LlmProvider::Local);

    let cloud_enabled = store
        .get("cloudEnabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let local_model_path = store
        .get("localModelPath")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    ProviderSettings {
        provider,
        cloud_enabled,
        local_model_path,
    }
}

fn default_settings() -> ProviderSettings {
    ProviderSettings {
        provider: LlmProvider::Local,
        cloud_enabled: false,
        local_model_path: None,
    }
}
