use gtk_layer_shell::Edge;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

// Main configuration structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub dock: DockConfig,
    #[serde(default)]
    pub calendar: CalendarConfig,
}

// Default implementation for Config
impl Default for Config {
    fn default() -> Self {
        Self {
            dock: DockConfig::default(),
            calendar: CalendarConfig::default(),
        }
    }
}

// Dock configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct DockConfig {
    pub enabled: bool,
    pub edge: EdgeConfig,
    pub hide_timeout: u64, // in milliseconds
}

// Default implementation for DockConfig
impl Default for DockConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default
            edge: EdgeConfig::Bottom,
            hide_timeout: 300,
        }
    }
}

// Calendar configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarConfig {
    pub enabled: bool,
    pub position: Position,
    pub size: Size,
}

// Default implementation for CalendarConfig
impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Enabled by default
            position: Position { x: 25, y: 25 },
            size: Size {
                width: 300,
                height: 250,
            },
        }
    }
}

// Edge configuration - which edge to attach widgets to
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum EdgeConfig {
    Left,
    Right,
    Top,
    Bottom,
}

// Convert EdgeConfig to GTK Layer Shell Edge
impl EdgeConfig {
    pub fn to_edge(&self) -> Edge {
        match self {
            EdgeConfig::Left => Edge::Left,
            EdgeConfig::Right => Edge::Right,
            EdgeConfig::Top => Edge::Top,
            EdgeConfig::Bottom => Edge::Bottom,
        }
    }
}

// Position configuration
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

// Size configuration
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

// Loads the configuration file or creates a default one if it doesn't exist
pub fn load_config() -> Config {
    let config_path = get_config_path();

    // Ensure config directory exists
    if let Some(config_dir) = config_path.parent() {
        if !config_dir.exists() {
            if let Err(e) = fs::create_dir_all(config_dir) {
                error!("Failed to create config directory: {}", e);
                return Config::default();
            }
        }
    }

    // Try to load the config file
    if config_path.exists() {
        match File::open(&config_path) {
            Ok(mut file) => {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    match toml::from_str::<Config>(&contents) {
                        Ok(config) => {
                            info!("Configuration loaded from {}", config_path.display());
                            return config;
                        }
                        Err(e) => {
                            error!("Failed to parse config file: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to open config file: {}", e);
            }
        }
    }

    // If we get here, either the file doesn't exist or there was an error
    // Create a default config and save it
    let default_config = Config::default();
    save_config(&default_config);
    default_config
}

// Saves the configuration to the config file
pub fn save_config(config: &Config) -> bool {
    let config_path = get_config_path();

    // Create config directory if needed
    if let Some(config_dir) = config_path.parent() {
        if !config_dir.exists() {
            if let Err(e) = fs::create_dir_all(config_dir) {
                error!("Failed to create config directory: {}", e);
                return false;
            }
        }
    }

    // Serialize the config and write it to the file
    match toml::to_string_pretty(config) {
        Ok(config_str) => match File::create(&config_path) {
            Ok(mut file) => {
                if file.write_all(config_str.as_bytes()).is_ok() {
                    info!("Configuration saved to {}", config_path.display());
                    return true;
                }
            }
            Err(e) => {
                error!("Failed to create config file: {}", e);
            }
        },
        Err(e) => {
            error!("Failed to serialize config: {}", e);
        }
    }

    false
}

// Get the path to the config file
fn get_config_path() -> PathBuf {
    let mut path = if let Some(config_dir) = dirs::config_dir() {
        config_dir
    } else {
        error!("Failed to determine config directory, using current directory");
        PathBuf::from(".")
    };

    path.push("swaydgets");
    path.push("config.toml");
    path
}
