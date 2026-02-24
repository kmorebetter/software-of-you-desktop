use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub claude_path: String,
    pub plugin_path: String,
    pub plugin_path_override: Option<String>,
    pub output_path: String,
    pub theme: String,
    pub window: WindowConfig,
    pub onboarding_complete: bool,
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowConfig {
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub terminal_width_pct: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            claude_path: String::new(),
            plugin_path: "~/.local/share/software-of-you/plugin".to_string(),
            plugin_path_override: None,
            output_path: "~/.local/share/software-of-you/output".to_string(),
            theme: "dark".to_string(),
            window: WindowConfig::default(),
            onboarding_complete: false,
            version: "1.0.0".to_string(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1400.0,
            height: 900.0,
            x: None,
            y: None,
            terminal_width_pct: 60.0,
        }
    }
}

pub fn config_path() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|d| d.join("software-of-you").join("config.json"))
        .ok_or_else(|| "Could not determine config directory".to_string())
}

#[tauri::command]
pub fn get_config() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
