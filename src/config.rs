use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    Y,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    Asc,
    Desc,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NlConfig {
    pub primary_axis: Axis,
    pub sort_primary: Sort,
    pub sort_secondary: Sort,
    pub active_panels: Vec<usize>,
    pub token_file_path: PathBuf,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub nl_config: NlConfig,
    pub audio_device: String,
    pub min_freq: u32,
    pub max_freq: u32,
    pub default_gain: f32,
    pub transition_time: u16,
    pub hues: Vec<u16>,
}

pub fn try_read_from_file(config_file_path: &Path) -> Result<Option<Config>, anyhow::Error> {
    if Path::try_exists(config_file_path)? {
        let mut config_file = File::open(config_file_path)?;
        let mut toml_str = String::new();
        config_file.read_to_string(&mut toml_str)?;

        match toml::from_str(&toml_str) {
            Ok(deserialized_config) => Ok(Some(deserialized_config)),
            Err(e) => Err(anyhow::Error::msg(format!(
                "Parsing the config file failed: {}",
                e
            ))),
        }
    } else {
        Ok(None)
    }
}

pub fn make_new_config_file(
    config_to_serialize: &Config,
    config_file: &Path,
) -> Result<(), anyhow::Error> {
    let config_dir = match config_file.parent() {
        Some(parent) => parent,
        None => {
            return Err(anyhow::Error::msg(format!(
                "Path '{}' is invalid",
                config_file.to_string_lossy()
            )));
        }
    };

    let config_toml = toml::to_string_pretty(&config_to_serialize)?;
    if !Path::try_exists(config_dir)? {
        fs::create_dir(config_dir)?;
    }
    let mut config_file = File::create(config_file)?;
    config_file.write_all(config_toml.as_bytes())?;

    Ok(())
}
