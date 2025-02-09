use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use log;

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
        
        // Create keyboards directory during first load
        create_keyboards_directory()?;
        
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
    let config_path = format!("{}/.config/clickclack/clickclack.toml", home);
    println!("Config path: {}", config_path);
    Ok(PathBuf::from(config_path))
}

// Add this new function
fn create_keyboards_directory() -> Result<PathBuf> {
    let home = std::env::var("HOME").expect("Failed to get HOME directory");
    let keyboards_path = PathBuf::from(format!("{}/.config/clickclack/keyboards", home));
    
    if !keyboards_path.exists() {
        fs::create_dir_all(&keyboards_path)?;
        log::info!("Created keyboards directory at {:?}", keyboards_path);
    }
    
    Ok(keyboards_path)
} 