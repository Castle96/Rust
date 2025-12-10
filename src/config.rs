use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub theme: Option<String>,
}

fn default_config_dir() -> PathBuf {
    if let Ok(p) = env::var("APPLE_CONFIG_PATH") {
        return PathBuf::from(p);
    }
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("apple");
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".config").join("apple");
    }
    PathBuf::from(".").join(".config").join("apple")
}

pub fn config_path() -> PathBuf {
    default_config_dir().join("config.json")
}

pub fn load_config() -> Config {
    let path = config_path();
    if let Ok(s) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<Config>(&s) {
            return cfg;
        }
    }
    Config::default()
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).context("creating config dir")?;
    }
    let s = serde_json::to_string_pretty(cfg).context("serialize config")?;
    fs::write(&path, s).context("write config")?;
    Ok(())
}
