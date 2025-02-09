use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub volume: f32,
    pub keyboard_profile: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 1.0,
            keyboard_profile: String::from("Kandas-Woods-v1"),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        
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
        fs::write(config_path, toml)?;
        Ok(())
    }
}

fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "clickclack", "clickclack")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;
    
    Ok(proj_dirs.config_dir().join("clickclack.toml"))
} 