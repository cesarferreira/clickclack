use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use log;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub volume: f32,
    pub switch_type: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 1.0,
            switch_type: String::from("mxblue"),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        
        // Create switchtypes directory during first load
        create_switchtypes_directory()?;
        
        if !config_path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)?;
        Ok(toml::from_str(&contents)?)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = get_config_path()?;
        
        // Ensure the config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml = toml::to_string_pretty(self)?;
        fs::write(config_path.clone(), toml)?;
        
        // Log the path where we saved the config
        log::info!("Configuration saved to {:?}", config_path);
        Ok(())
    }
}

fn get_config_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").expect("Failed to get HOME directory");
    Ok(PathBuf::from(home).join(".config/clickclack/config.toml"))
}

fn create_switchtypes_directory() -> Result<()> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("clickclack")
        .join("switchtypes");
    
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    
    Ok(())
} 