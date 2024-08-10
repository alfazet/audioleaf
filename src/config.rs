use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::{self, File};
use std::io::prelude::*;
use std::net::Ipv4Addr;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub nl_config: NlConfig,
    pub fifo_path: String,
    pub sample_rate: usize,
    pub n_samples: usize,
    pub max_volume_level: f32,
    pub brightness_range: f32,
    pub freq_ranges: Vec<FreqRange>,
}

impl Config {
    pub fn new(given_ip: Option<Ipv4Addr>) -> Result<Config, Box<dyn Error>> {
        let Some(system_config_dir_path) = dirs::config_dir() else {
            return Err("Config directory not found on your system.".into());
        };
        let config_dir_path = system_config_dir_path.join("audioleaf");
        let config_file_path = config_dir_path.join("audioleaf.toml");

        let config = if Path::try_exists(&config_file_path)? {
            let mut config_file = File::open(config_file_path)?;
            let mut toml_str = String::new();
            config_file.read_to_string(&mut toml_str)?;

            toml::from_str(&toml_str)?
        } else {
            let ip = match given_ip {
                Some(ip) => ip,
                None => {
                    return Err("It seems you're running audioleaf for the first time. Please provide the IP of your Nanoleaf device. For more information consult the README.".into());
                }
            };
            let nl_config = NlConfig {
                ip: ip.to_string(),
                port: 6789,
                token_file_path: "~/.config/audioleaf/nltoken".to_string(),
                active_panels: vec![0, 1, 2, 6, 9, 10, 11],
                primary_axis: Axis::Y,
                sort_primary: Sort::Asc,
                sort_secondary: Sort::Asc,
                trans_time: 2,
            };
            let fifo_path = "/tmp/mpd.fifo".to_string();
            let sample_rate = 48_000;
            let n_samples = 2048;
            let max_volume_level = 13.0;
            let brightness_range = 75.0;
            let cutoffs = vec![60, 250, 500, 2000, 4000, 6000, sample_rate];
            let colors = vec![
                "#fffb00".to_string(),
                "#ff8000".to_string(),
                "#ff0040".to_string(),
                "#ff00bf".to_string(),
                "#bf00ff".to_string(),
                "#4000ff".to_string(),
                "#0040ff".to_string(),
            ];
            let freq_ranges = cutoffs
                .into_iter()
                .zip(colors)
                .map(|(cutoff, color)| FreqRange { cutoff, color })
                .collect::<Vec<_>>();

            Config {
                nl_config,
                fifo_path,
                sample_rate,
                n_samples,
                max_volume_level,
                brightness_range,
                freq_ranges,
            }
            // let config_toml = toml::to_string_pretty(&config_to_serialize)?;
            //
            // if !Path::try_exists(&config_dir_path)? {
            //     fs::create_dir(&config_dir_path)?;
            // }
            // let mut config_file = File::create(config_file_path)?;
            // config_file.write_all(config_toml.as_bytes())?;
        };

        Ok(config)
    }

    pub fn make_new_config_file(config_to_serialize: Config) -> Result<(), Box<dyn Error>> {
        let Some(system_config_dir_path) = dirs::config_dir() else {
            return Err("Config directory not found on your system.".into());
        };
        let config_dir_path = system_config_dir_path.join("audioleaf");
        let config_file_path = config_dir_path.join("audioleaf.toml");

        let config_toml = toml::to_string_pretty(&config_to_serialize)?;
        if !Path::try_exists(&config_dir_path)? {
            fs::create_dir(&config_dir_path)?;
        }
        let mut config_file = File::create(config_file_path)?;
        config_file.write_all(config_toml.as_bytes())?;

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FreqRange {
    pub cutoff: usize,
    pub color: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NlConfig {
    pub ip: String,
    pub port: u16,
    pub token_file_path: String,
    pub active_panels: Vec<usize>,
    pub primary_axis: Axis,
    pub sort_primary: Sort,
    pub sort_secondary: Sort,
    pub trans_time: u16,
}

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
