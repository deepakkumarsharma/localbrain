mod persistence;

use crate::embeddings;
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
    pub embedding_model_path: Option<String>,
    pub last_project_path: Option<String>,
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
                embedding_model_path: None,
                last_project_path: None,
            }),
        }
    }

    pub fn load_from_disk(&self, app: &AppHandle) {
        let loaded = persistence::load_settings(app);
        embeddings::set_embedding_model_path(loaded.embedding_model_path.clone());
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
        let mut settings = self.settings.lock().map_err(|error| error.to_string())?;
        settings.provider = provider;
        settings.cloud_enabled = cloud_enabled && provider != LlmProvider::Local;
        let updated = settings.clone();
        drop(settings);
        persistence::save_settings(app, &updated)?;

        Ok(updated)
    }

    pub fn set_local_model_path(
        &self,
        app: &AppHandle,
        local_model_path: Option<String>,
    ) -> Result<ProviderSettings, String> {
        let mut settings = self.settings.lock().map_err(|error| error.to_string())?;
        settings.local_model_path = local_model_path;
        let updated = settings.clone();
        drop(settings);
        persistence::save_settings(app, &updated)?;

        Ok(updated)
    }

    pub fn set_embedding_model_path(
        &self,
        app: &AppHandle,
        embedding_model_path: Option<String>,
    ) -> Result<ProviderSettings, String> {
        let updated = {
            let settings = self.settings.lock().map_err(|error| error.to_string())?;
            let mut updated = settings.clone();
            updated.embedding_model_path = embedding_model_path;
            updated
        };
        persistence::save_settings(app, &updated)?;
        embeddings::set_embedding_model_path(updated.embedding_model_path.clone());
        if let Ok(mut settings) = self.settings.lock() {
            *settings = updated.clone();
        }

        Ok(updated)
    }

    pub fn set_last_project_path(
        &self,
        app: &AppHandle,
        last_project_path: Option<String>,
    ) -> Result<ProviderSettings, String> {
        let mut settings = self.settings.lock().map_err(|error| error.to_string())?;
        settings.last_project_path = last_project_path;
        let updated = settings.clone();
        drop(settings);
        persistence::save_settings(app, &updated)?;
        Ok(updated)
    }
}
