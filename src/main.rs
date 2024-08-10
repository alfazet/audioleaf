#![allow(unreachable_code)]

use config::{Axis, Config, NlConfig, Sort};
use nanoleaf::{Command, Nanoleaf, Panel};
use std::env;
use std::error::Error;
use std::net::Ipv4Addr;

mod audio;
mod config;
mod fft;
mod nanoleaf;

fn main() -> Result<(), Box<dyn Error>> {
    let cmd_args = env::args().collect::<Vec<String>>();
    let given_ip = if cmd_args.len() > 1 {
        Some(cmd_args[1].parse::<Ipv4Addr>()?)
    } else {
        None
    };
    let config = Config::new(given_ip)?;
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
        active_panels,
        primary_axis,
        sort_primary,
        sort_secondary,
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
        // Why can we make a new config only here, and not earlier? Because before making it we need to ensure that
        // the given IP address is fine and that we actually connect to a Nanoleaf device successfully.
        Config::make_new_config_file(config_clone)?;
        println!("Connected to a Nanoleaf device with local address {} and finished setup. You can now run audioleaf again.", given_ip);
        return Ok(());
    }

    nl.sort_panels(|a: &Panel, b: &Panel| sort_func(*a, *b));
    loop {
        let samples = audio::get_samples(&fifo_path, n_samples)?;
        let spectrum = audio::freq_domain(&samples);
        let colors_to_apply = audio::visualise(
            spectrum,
            &freq_ranges,
            active_panels.len(),
            sample_rate,
            max_volume_level,
            brightness_range,
        )?;
        let commands = active_panels
            .iter()
            .zip(colors_to_apply)
            .map(|(panel_no, color)| Command {
                panel_no: *panel_no,
                color,
                trans_time,
            })
            .collect::<Vec<_>>();
        nl.run_commands(&commands)?;
    }

    Ok(())
}
