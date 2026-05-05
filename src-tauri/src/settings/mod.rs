use serde::{Deserialize, Serialize};
use std::sync::Mutex;

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
            }),
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
        provider: LlmProvider,
        cloud_enabled: bool,
    ) -> Result<ProviderSettings, String> {
        let mut settings = self.settings.lock().map_err(|error| error.to_string())?;
        settings.provider = provider;
        settings.cloud_enabled = cloud_enabled && provider != LlmProvider::Local;

        Ok(settings.clone())
    }
}
