use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use std::process::Command;
use log::info;
use plist::Value;

const PLIST_LABEL: &str = "com.clickclack.daemon";

pub struct ServiceManager {
    plist_path: PathBuf,
}

impl ServiceManager {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME").context("Failed to get HOME directory")?;
        let plist_path = PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", PLIST_LABEL));

        Ok(Self { plist_path })
    }

    pub fn install_service(&self) -> Result<()> {
        info!("Installing ClickClack service...");
        
        // Get the path to the current executable
        let exe_path = std::env::current_exe()
            .context("Failed to get current executable path")?;
        
        // Create LaunchAgent directory if it doesn't exist
        if let Some(parent) = self.plist_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let home = std::env::var("HOME").context("Failed to get HOME directory")?;

        // Create the plist content
        let dict = vec![
            (String::from("Label"), Value::String(PLIST_LABEL.into())),
            (String::from("ProgramArguments"), Value::Array(vec![
                Value::String(exe_path.to_string_lossy().into_owned())
            ])),
            (String::from("RunAtLoad"), Value::Boolean(true)),
            (String::from("KeepAlive"), Value::Boolean(true)),
            (String::from("StandardOutPath"), Value::String(format!("{}/Library/Logs/clickclack.log", home))),
            (String::from("StandardErrorPath"), Value::String(format!("{}/Library/Logs/clickclack.error.log", home))),
        ];
        
        let plist = Value::Dictionary(dict.into_iter().collect());

        // Write the plist file
        plist::to_file_xml(&self.plist_path, &plist)?;
        info!("Service plist created at: {:?}", self.plist_path);

        Ok(())
    }

    pub fn start_service(&self) -> Result<()> {
        info!("Starting ClickClack service...");
        if !self.plist_path.exists() {
            self.install_service()?;
        }
        
        // Load the service using launchctl
        let output = Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&self.plist_path)
            .output()
            .context("Failed to execute launchctl load command")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to start service: {}", error);
        }

        info!("Service started successfully");
        Ok(())
    }

    pub fn stop_service(&self) -> Result<()> {
        info!("Stopping ClickClack service...");
        if self.plist_path.exists() {
            // Unload the service using launchctl
            let output = Command::new("launchctl")
                .args(["unload", "-w"])
                .arg(&self.plist_path)
                .output()
                .context("Failed to execute launchctl unload command")?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to stop service: {}", error);
            }

            info!("Service stopped successfully");
        } else {
            info!("Service is not installed");
        }
        Ok(())
    }

    pub fn restart_service(&self) -> Result<()> {
        info!("Restarting ClickClack service...");
        self.stop_service()?;
        self.start_service()?;
        info!("Service restarted successfully");
        Ok(())
    }

    pub fn is_service_running(&self) -> bool {
        if !self.plist_path.exists() {
            return false;
        }

        // Check service status using launchctl
        let output = Command::new("launchctl")
            .args(["list"])
            .output()
            .ok();

        if let Some(output) = output {
            if output.status.success() {
                let output = String::from_utf8_lossy(&output.stdout);
                return output.lines().any(|line| line.contains(PLIST_LABEL));
            }
        }

        false
    }
} 