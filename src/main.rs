#![allow(unreachable_code)]

use config::{Axis, Config, NlConfig, Sort};
use nanoleaf::{Command, Nanoleaf, Panel};
use std::collections::VecDeque;
use std::env;
use std::error::Error;
use std::net::Ipv4Addr;

mod audio;
mod config;
mod fft;
mod nanoleaf;

const WINDOW_LEN: usize = 5;

fn main() -> Result<(), Box<dyn Error>> {
    let cmd_args = env::args().collect::<Vec<String>>();
    let any_args = cmd_args.len() > 1;
    if any_args {
        let arg = cmd_args[1].parse::<String>().unwrap();
        if &arg == "--help" || &arg == "-h" {
            return Err("If you're running audioleaf for the first time, please run `audioleaf <ip_address>` with the local IP address of your Nanoleaf device while its contol lights are flashing. If you've already done the setup, please run audioleaf with no arguments. For more details see the README at https://github.com/alfazet/audioleaf.".into());
        }
    }
    let given_ip = if any_args {
        let parsed = match cmd_args[1].parse::<Ipv4Addr>() {
            Ok(parsed) => parsed,
            Err(_) => {
                return Err(format!("Couldn't parse IP address '{}'.", cmd_args[1]).into());
            }
        };
        Some(parsed)
    } else {
        None
    };
    let config = match Config::new(given_ip) {
        Ok(config) => config,
        Err(e) => {
            return Err(format!("Generating a default configuration failed. {}", e).into());
        }
    };
    let config_clone = config.clone();
    let Config {
        nl_config,
        fifo_path,
        sample_rate,
        n_samples,
        max_volume_level,
        brightness_range,
        freq_ranges,
    } = config;
    let NlConfig {
        ip,
        port,
        token_file_path,
        primary_axis,
        sort_primary,
        sort_secondary,
        active_panels,
        trans_time,
    } = nl_config;
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

    let mut nl = match Nanoleaf::new(&ip, port, &token_file_path) {
        Ok(nl) => nl,
        Err(_) => {
            return Err("Couldn't connect to the Nanoleaf device. Make sure that the IP address is correct and that you're running this command WHILE the control lights are flashing.".into());
        }
    };
    if let Some(given_ip) = given_ip {
        match Config::make_new_config_file(config_clone) {
            Ok(_) => (),
            Err(e) => {
                return Err(format!("Making a new config file failed. {}", e).into());
            }
        };
        println!("Connected to a Nanoleaf device at local IP {}, finished setup and created the config file. You can now run audioleaf again.", given_ip);
        return Ok(());
    }

    nl.sort_panels(|a: &Panel, b: &Panel| sort_func(*a, *b));
    let reference_base_level = max_volume_level.div_euclid(2.0);
    let mut window = VecDeque::<f32>::from([0.0; WINDOW_LEN]);
    loop {
        let samples = match audio::get_samples(&fifo_path, n_samples) {
            Ok(samples) => samples,
            Err(e) => {
                return Err(format!("Reading PCM samples failed. {}", e).into());
            }
        };
        let spectrum = audio::freq_domain(&samples);

        window.rotate_left(1);
        *window.back_mut().unwrap() = spectrum.iter().sum::<f32>() / (n_samples as f32);
        let window_avg = window.iter().sum::<f32>() / (WINDOW_LEN as f32);
        let cur_base_level = reference_base_level + 0.2 * (window_avg - reference_base_level);
        let colors_to_apply = match audio::visualise(
            spectrum,
            &freq_ranges,
            active_panels.len(),
            sample_rate,
            cur_base_level,
            max_volume_level,
            brightness_range,
        ) {
            Ok(colors_to_apply) => colors_to_apply,
            Err(e) => {
                return Err(format!("Visualizing failed. {}", e).into());
            }
        };
        let commands = active_panels
            .iter()
            .zip(colors_to_apply)
            .map(|(panel_no, color)| Command {
                panel_no: *panel_no,
                color,
                trans_time,
            })
            .collect::<Vec<_>>();
        match nl.run_commands(&commands) {
            Ok(_) => (),
            Err(e) => {
                return Err(format!("Running Nanoleaf commands failed. {}", e).into());
            }
        };
    }

    Ok(())
}
