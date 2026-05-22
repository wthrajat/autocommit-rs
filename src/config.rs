use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::types::{MessageStyle, ModelType};

const CONFIG_FILE: &str = ".autocommitrc";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub openai_key: String,
    #[serde(default)]
    pub gemini_key: String,
    #[serde(default)]
    pub model: ModelType,
    #[serde(default)]
    pub message_style: MessageStyle,
    #[serde(default)]
    pub signed_commit: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_key: String::new(),
            gemini_key: String::new(),
            model: ModelType::Openai,
            message_style: MessageStyle::Short,
            signed_commit: false,
        }
    }
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(CONFIG_FILE)
}

pub fn load_config() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: Config = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path();
    let data = serde_json::to_string_pretty(config)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&path, &data)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, &data)?;
    }
    Ok(())
}

pub fn get_config() -> Result<Config> {
    let mut config = load_config().unwrap_or_default();

    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        config.gemini_key = key;
        config.model = ModelType::Gemini;
    } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        config.openai_key = key;
        config.model = ModelType::Openai;
    }

    if let Ok(val) = std::env::var("AUTOCOMMIT_MODEL") {
        match val.as_str() {
            "gemini" => config.model = ModelType::Gemini,
            "openai" => config.model = ModelType::Openai,
            _ => {}
        }
    }

    if let Ok(val) = std::env::var("AUTOCOMMIT_MESSAGE_STYLE") {
        match val.as_str() {
            "short" => config.message_style = MessageStyle::Short,
            "long" => config.message_style = MessageStyle::Long,
            _ => {}
        }
    }

    Ok(config)
}

pub fn config_file_exists() -> bool {
    let config = load_config().unwrap_or_default();
    !config.openai_key.is_empty() || !config.gemini_key.is_empty()
}

pub fn save_api_key(api_key: &str, model: ModelType) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    match model {
        ModelType::Openai => config.openai_key = api_key.to_string(),
        ModelType::Gemini => config.gemini_key = api_key.to_string(),
    }
    config.model = model;
    save_config(&config)
}

pub fn set_model(model: ModelType) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.model = model;
    save_config(&config)
}

pub fn set_message_style(style: MessageStyle) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.message_style = style;
    save_config(&config)
}

pub fn set_signed_commit(signed: bool) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.signed_commit = signed;
    save_config(&config)
}

pub fn get_message_style(config: &Config) -> MessageStyle {
    if let Ok(val) = std::env::var("AUTOCOMMIT_MESSAGE_STYLE") {
        match val.as_str() {
            "short" => return MessageStyle::Short,
            "long" => return MessageStyle::Long,
            _ => {}
        }
    }
    config.message_style
}

pub fn get_signed_commit(config: &Config) -> bool {
    config.signed_commit
}
