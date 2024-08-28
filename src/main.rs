use clap::Parser;
use config::{Axis, Config, NlConfig, Sort};
use console::Term;
use core::f32;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, StreamConfig};
use nanoleaf::{Command, Nanoleaf, Panel};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

mod audio;
mod config;
mod nanoleaf;

const SAMPLING_ITERS: u8 = 3;

/// Audioleaf - An audio visualizer for Nanoleaf Canvas
#[derive(Parser, Debug)]
#[command(version, about, author, long_about = None)]
struct CmdOpt {
    /// Path of the configuration file
    #[arg(short, long)]
    config_file: Option<PathBuf>,

    /// Path of the file contatining the Nanoleaf auth token
    #[arg(short, long)]
    token_file: Option<PathBuf>,

    /// Local IP address of the Nanoleaf device - must be specified when running audioleaf for the first time!
    #[arg(long)]
    ip: Option<String>,

    /// Port to which a UDP socket connected to the Nanoleaf device will be bound
    #[arg(short, long)]
    port: Option<u16>,

    /// Audio input device to serve as the source of audio data
    #[arg(short, long)]
    audio_device: Option<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let CmdOpt {
        config_file: config_file_path,
        token_file: token_file_path,
        ip: nl_ip,
        port: nl_port,
        audio_device: device_name,
    } = CmdOpt::parse();
    let config_file_path =
        config_file_path.unwrap_or(dirs::config_dir().unwrap().join("audioleaf/audioleaf.toml"));

    let (config, mut nl) = match config::try_read_from_file(&config_file_path)? {
        Some(mut config) => {
            config.nl_config.token_file_path =
                token_file_path.unwrap_or(config.nl_config.token_file_path);
            config.nl_config.ip = nl_ip.unwrap_or(config.nl_config.ip);
            config.nl_config.port = nl_port.unwrap_or(config.nl_config.port);
            config.audio_device = device_name.unwrap_or(config.audio_device);
            if config.hues.is_empty() {
                config.hues.push(0);
            }
            if config.nl_config.active_panels.is_empty() {
                config.nl_config.active_panels.push(1);
            }

            println!(
                "Connecting to Nanoleaf device at {}...",
                config.nl_config.ip
            );
            let nl = match Nanoleaf::new(
                &config.nl_config.ip,
                config.nl_config.port,
                &config.nl_config.token_file_path,
            ) {
                Ok(nl) => nl,
                Err(e) => {
                    return Err(anyhow::Error::msg(format!(
                        "Connection to Nanoleaf failed: {}",
                        e
                    )));
                }
            };

            let max_active_panel = *config.nl_config.active_panels.iter().max().unwrap();
            if nl.panels.len() < max_active_panel {
                return Err(anyhow::Error::msg(format!("Panel {} in active_panels, but only {} panels overall", max_active_panel, nl.panels.len())));
            }
            if config.nl_config.active_panels.iter().any(|&x| x == 0) {
                return Err(anyhow::Error::msg("Panel 0 is invalid, panels should be numbered from 1"));
            }
            println!("Success!");

            (config, nl)
        }
        None => {
            println!(
                "Config file '{}' not found",
                config_file_path.to_string_lossy()
            );
            if nl_ip.is_none()
                || nl_ip
                    .as_ref()
                    .is_some_and(|ip| ip.parse::<Ipv4Addr>().is_err())
            {
                return Err(anyhow::Error::msg(
                    "IP unspecified or invalid, please run `audioleaf <nanoleaf_device_ip>`",
                ));
            }
            let nl_ip = nl_ip.unwrap();
            let nl_port = nl_port.unwrap_or(6789);
            let token_file_path =
                token_file_path.unwrap_or(dirs::config_dir().unwrap().join("audioleaf/nltoken"));
            println!("Connecting to Nanoleaf device at {}...", nl_ip);
            let nl = match Nanoleaf::new(&nl_ip, nl_port, &token_file_path) {
                Ok(nl) => nl,
                Err(e) => {
                    return Err(anyhow::Error::msg(format!(
                        "Connection to Nanoleaf failed: {}",
                        e
                    )));
                }
            };
            println!("Success!");
            let nl_config = NlConfig {
                primary_axis: Axis::Y,
                sort_primary: Sort::Asc,
                sort_secondary: Sort::Asc,
                active_panels: (1..=nl.panels.len()).collect::<Vec<_>>(),
                token_file_path,
                ip: nl_ip,
                port: nl_port,
            };

            let config = Config {
                nl_config,
                audio_device: device_name.unwrap_or(String::from("default")),
                min_freq: 20,
                max_freq: 20_000,
                default_gain: 3.0 / 8.0,
                hues: (240..=420)
                    .rev()
                    .step_by(180 / (nl.panels.len() - 1))
                    .map(|x| x % 360)
                    .collect::<Vec<u16>>(),
            };

            config::make_new_config_file(&config, &config_file_path)?;
            println!(
                "Created config file '{}'",
                config_file_path.to_string_lossy()
            );

            (config, nl)
        }
    };

    let Config {
        nl_config,
        audio_device: device_name,
        min_freq,
        max_freq,
        default_gain,
        mut hues,
    } = config;
    let NlConfig {
        primary_axis,
        sort_primary,
        sort_secondary,
        active_panels,
        ..
    } = nl_config;

    while hues.len() < active_panels.len() {
        hues.push(*hues.last().unwrap());
    }
    while hues.len() > active_panels.len() {
        hues.pop();
    }

    let sort_func = match primary_axis {
        Axis::X => match (sort_primary, sort_secondary) {
            (Sort::Asc, Sort::Asc) => |lhs: Panel, rhs: Panel| (lhs.x, lhs.y).cmp(&(rhs.x, rhs.y)),
            (Sort::Asc, Sort::Desc) => {
                |lhs: Panel, rhs: Panel| (lhs.x, -lhs.y).cmp(&(rhs.x, -rhs.y))
            }
            (Sort::Desc, Sort::Asc) => {
                |lhs: Panel, rhs: Panel| (-lhs.x, lhs.y).cmp(&(-rhs.x, rhs.y))
            }
            (Sort::Desc, Sort::Desc) => {
                |lhs: Panel, rhs: Panel| (-lhs.x, -lhs.y).cmp(&(-rhs.x, -rhs.y))
            }
        },
        Axis::Y => match (sort_primary, sort_secondary) {
            (Sort::Asc, Sort::Asc) => |lhs: Panel, rhs: Panel| (lhs.y, lhs.x).cmp(&(rhs.y, rhs.x)),
            (Sort::Asc, Sort::Desc) => {
                |lhs: Panel, rhs: Panel| (lhs.y, -lhs.x).cmp(&(rhs.y, -rhs.x))
            }
            (Sort::Desc, Sort::Asc) => {
                |lhs: Panel, rhs: Panel| (-lhs.y, lhs.x).cmp(&(-rhs.y, rhs.x))
            }
            (Sort::Desc, Sort::Desc) => {
                |lhs: Panel, rhs: Panel| (-lhs.y, -lhs.x).cmp(&(-rhs.y, -rhs.x))
            }
        },
    };
    nl.sort_panels(|a: &Panel, b: &Panel| sort_func(*a, *b));

    let host = cpal::default_host();
    let device = match device_name.as_str() {
        "default" => host.default_input_device(),
        _ => host
            .input_devices()?
            .find(|x| x.name().map(|y| y == device_name).unwrap_or(false)),
    };
    let device = match device {
        Some(device) => device,
        None => {
            return Err(anyhow::Error::msg(format!(
                "Input device '{}' not found, list of available input devices: {}",
                device_name,
                host.input_devices()?.fold(String::new(), |acc, dev| acc
                    + &dev.name().unwrap_or_default()
                    + ", ")
            )));
        }
    };
    let audio_config: StreamConfig = device.default_input_config()?.into();
    if max_freq > audio_config.sample_rate.0 / 2 {
        return Err(anyhow::Error::msg(format!(
            "Maximal frequency to visualize ({} Hz) is more than half of the sample rate ({} Hz)",
            max_freq, audio_config.sample_rate.0
        )));
    }

    let (tx_audio, rx) = mpsc::channel();
    let tx_user_input = tx_audio.clone();
    let error_callback =
        move |err| eprintln!("An error has occured while playing the stream: {}", err);
    let data_callback = move |data: &[f32], _: &InputCallbackInfo| {
        let mut samples = Vec::new();
        let n_channels = audio_config.channels as usize;
        for chunk in data.chunks_exact(n_channels) {
            // max of samples from all channels
            samples.push(
                chunk
                    .iter()
                    .fold(f32::NEG_INFINITY, |acc, x| f32::max(acc, *x)),
            );
        }
        tx_audio.send(Some(samples)).unwrap();
    };
    let stream = device.build_input_stream(&audio_config, data_callback, error_callback, None)?;
    stream.play()?;

    let gain_original = Arc::new(Mutex::new(default_gain));
    let gain = Arc::clone(&gain_original);
    let visualizer_thread = thread::spawn(move || {
        'visualizer_loop: loop {
            let mut time_samples = Vec::new();
            // we need to take samples a couple of times because Nanoleaf can't change colors faster than every 100 ms
            for _ in 0..SAMPLING_ITERS {
                match rx.recv().unwrap() {
                    Some(mut samples) => time_samples.append(&mut samples),
                    None => break 'visualizer_loop,
                }
            }
            let freq_samples = audio::process(time_samples, *gain.lock().unwrap());

            let hz_per_bin = (audio_config.sample_rate.0 / 2) / (freq_samples.len() as u32);
            let colors = audio::visualize(freq_samples, min_freq, max_freq, &hues, hz_per_bin);
            let commands = active_panels
                .iter()
                .zip(colors)
                .map(|(panel_no, color)| Command {
                    panel_no: *panel_no,
                    color,
                })
                .collect::<Vec<_>>();
            nl.run_commands(commands).unwrap();
        }
    });

    let gain = Arc::clone(&gain_original);
    let stdout = Term::buffered_stdout();
    'main_loop: loop {
        if let Ok(ch) = stdout.read_char() {
            match ch {
                'Q' => {
                    tx_user_input.send(None).unwrap();
                    visualizer_thread.join().unwrap();
                    println!("Quitting audioleaf...");
                    break 'main_loop;
                }
                '=' => {
                    let mut gain = gain.lock().unwrap();
                    *gain += 0.1;
                }
                '-' => {
                    let mut gain = gain.lock().unwrap();
                    *gain -= 0.1;
                    if (*gain).is_sign_negative() {
                        *gain = 0.0;
                    }
                }
                _ => (),
            }
        }
    }

    Ok(())
}
