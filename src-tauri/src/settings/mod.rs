pub mod persistence;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LlmProvider {
    Local,
    Anthropic,
    Gemini,
    OpenAi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    pub provider: LlmProvider,
    pub cloud_enabled: bool,
    pub local_model_path: Option<String>,
}

pub struct SettingsStore {
    settings: Mutex<ProviderSettings>,
}

impl SettingsStore {
    pub fn new() -> Self {
        Self {
            settings: Mutex::new(ProviderSettings {
                provider: LlmProvider::Local,
                cloud_enabled: false,
                local_model_path: None,
            }),
        }
    }

    pub fn load_from_disk(&self, app: &AppHandle) {
        let loaded = persistence::load_settings(app);
        if let Ok(mut settings) = self.settings.lock() {
            *settings = loaded;
        }
    }

    pub fn get(&self) -> Result<ProviderSettings, String> {
        self.settings
            .lock()
            .map(|settings| settings.clone())
            .map_err(|error| error.to_string())
    }

    pub fn set_provider(
        &self,
        app: &AppHandle,
        provider: LlmProvider,
        cloud_enabled: bool,
    ) -> Result<ProviderSettings, String> {
        let mut cloned = self.get()?;
        cloned.provider = provider;
        cloned.cloud_enabled = cloud_enabled && provider != LlmProvider::Local;
        persistence::save_settings(app, &cloned)?;
        let mut settings = self.settings.lock().map_err(|error| error.to_string())?;
        *settings = cloned.clone();
        Ok(cloned)
    }

    pub fn set_local_model_path(
        &self,
        app: &AppHandle,
        path: Option<String>,
    ) -> Result<ProviderSettings, String> {
        let mut cloned = self.get()?;
        cloned.local_model_path = path;
        persistence::save_settings(app, &cloned)?;
        let mut settings = self.settings.lock().map_err(|error| error.to_string())?;
        *settings = cloned.clone();
        Ok(cloned)
    }
}
