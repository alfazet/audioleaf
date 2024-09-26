use clap::Parser;
use config::{Axis, Config, NlConfig, Sort};
use console::Term;
use core::f32;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, Sample, StreamConfig};
use nanoleaf::{Command, Nanoleaf, Panel};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

mod audio;
mod config;
mod nanoleaf;

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
            // so that there's at least one color / one active panel
            if config.hues.is_empty() {
                config.hues.push(0);
            }
            if config.nl_config.active_panels.is_empty() {
                config.nl_config.active_panels.push(1);
            }

            println!(
                "Connecting to a Nanoleaf device at {}...",
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
                return Err(anyhow::Error::msg(format!(
                    "Panel {} is specified in active_panels, but there are only {} panels available",
                    max_active_panel,
                    nl.panels.len()
                )));
            }
            if config.nl_config.active_panels.iter().any(|&x| x == 0) {
                return Err(anyhow::Error::msg(
                    "Panels should be numbered starting from 1",
                ));
            }

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
                    "IP unspecified or invalid, please run `audioleaf --ip <nanoleaf_device_ip>`",
                ));
            }
            let nl_ip = nl_ip.unwrap();
            let nl_port = nl_port.unwrap_or(6789);
            let token_file_path =
                token_file_path.unwrap_or(dirs::config_dir().unwrap().join("audioleaf/nltoken"));
            println!("Connecting to a Nanoleaf device at {}...", nl_ip);
            let nl = match Nanoleaf::new(&nl_ip, nl_port, &token_file_path) {
                Ok(nl) => nl,
                Err(e) => {
                    return Err(anyhow::Error::msg(format!(
                        "Connection to Nanoleaf failed: {}",
                        e
                    )));
                }
            };

            // default config
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
                max_freq: 6000,
                default_gain: 0.5,
                transition_time: 2,
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
    println!("Connected to {}", nl.name);

    let Config {
        nl_config,
        audio_device: device_name,
        min_freq,
        max_freq,
        default_gain,
        transition_time,
        mut hues,
    } = config;
    let NlConfig {
        primary_axis,
        sort_primary,
        sort_secondary,
        active_panels,
        ..
    } = nl_config;

    // so that panels can be matched one-to-one with hues
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
                "Input device '{}' not found, available input devices: {}",
                device_name,
                host.input_devices()?.fold(String::new(), |acc, dev| acc
                    + &dev.name().unwrap_or_default()
                    + ", ")
            )));
        }
    };
    let audio_config = device.default_input_config()?;
    let sample_format = audio_config.sample_format();
    let audio_config: StreamConfig = audio_config.into();
    if max_freq > audio_config.sample_rate.0 / 2 {
        return Err(anyhow::Error::msg(format!(
            "Maximal frequency to visualize ({} Hz) must be less than half of the sample rate ({} Hz)",
            max_freq, audio_config.sample_rate.0
        )));
    }

    if transition_time < 1 {
        return Err(anyhow::Error::msg("Transition time must be positive"));
    }

    let (tx_audio, rx) = mpsc::channel();
    let tx_user_input = tx_audio.clone();
    let error_callback =
        move |err| eprintln!("An error has occured while playing the stream: {}", err);
    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &audio_config,
            move |data, _: &InputCallbackInfo| {
                data_callback(data.to_vec(), audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::F64 => device.build_input_stream(
            &audio_config,
            move |data: &[f64], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::I8 => device.build_input_stream(
            &audio_config,
            move |data: &[i8], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &audio_config,
            move |data: &[i16], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::I32 => device.build_input_stream(
            &audio_config,
            move |data: &[i32], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::I64 => device.build_input_stream(
            &audio_config,
            move |data: &[i64], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::U8 => device.build_input_stream(
            &audio_config,
            move |data: &[u8], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &audio_config,
            move |data: &[u16], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::U32 => device.build_input_stream(
            &audio_config,
            move |data: &[u32], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        cpal::SampleFormat::U64 => device.build_input_stream(
            &audio_config,
            move |data: &[u64], _: &InputCallbackInfo| {
                let data_f32 = Vec::from_iter(data.iter().map(|sample| sample.to_sample::<f32>()));
                data_callback(data_f32, audio_config.channels as usize, &tx_audio)
            },
            error_callback,
            None,
        )?,
        sample_format => {
            return Err(anyhow::Error::msg(format!(
                "Unsupported sample format: {}",
                sample_format
            )));
        }
    };
    stream.play()?;

    let gain_original = Arc::new(Mutex::new(default_gain));
    let gain = Arc::clone(&gain_original);
    let visualizer_thread = thread::spawn(move || {
        let mut colors = hues
            .into_iter()
            .map(|hue| palette::Hwb::new(hue as f32, 1.0, 0.0))
            .collect::<Vec<_>>();
        'visualizer_loop: loop {
            let mut time_samples = Vec::new();
            for _ in 0..(2 * transition_time) {
                match rx.recv().unwrap() {
                    Some(mut samples) => time_samples.append(&mut samples),
                    None => break 'visualizer_loop,
                }
            }
            let freq_samples = audio::process(time_samples, *gain.lock().unwrap());

            let hz_per_bin = (audio_config.sample_rate.0 / 2) / (freq_samples.len() as u32);
            audio::update_colors(&mut colors, freq_samples, min_freq, max_freq, hz_per_bin);
            let commands = active_panels
                .iter()
                .zip(colors.iter())
                .map(|(panel_no, color)| Command {
                    panel_no: *panel_no,
                    color: *color,
                    transition_time,
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
                    *gain += 0.05;
                }
                '-' => {
                    let mut gain = gain.lock().unwrap();
                    *gain -= 0.05;
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

fn data_callback(data: Vec<f32>, n_channels: usize, tx: &mpsc::Sender<Option<Vec<f32>>>) {
    let mut samples = Vec::new();
    for chunk in data.chunks_exact(n_channels) {
        // max of samples from all channels
        samples.push(
            chunk
                .iter()
                .fold(f32::NEG_INFINITY, |acc, x| f32::max(acc, *x)),
        );
    }
    tx.send(Some(samples)).unwrap();
}
