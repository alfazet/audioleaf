# Audioleaf

A music visualizer for Nanoleaf Canvas. Works best with MPD (the Music Player Daemon).

## Installation

Install from cargo using `cargo install audioleaf`. Make sure that the directory with cargo binaries (by default `$HOME\.cargo\bin`) is added to your PATH.

## Usage

>
IMPORTANT: When running audioleaf for the first time, do the following:
* Press and hold your Nanoleaf control panels's power button for about 5 seconds, until the icons start sequentially flashing.
* *While* the icons are flashing, run `audioleaf <local ip address of your Nanoleaf device>` (for example `audioleaf 192.168.0.101`). You can find that IP address by logging in to your WiFi router's page and navigating to the DHCP clients list. Your Nanoleaf will be the device whose MAC address looks like `00:55:DA:5X:XX:XX`. Simply copy the assigned IP address from there.
* If everything went correctly, audioleaf will output `Connection established successfully, token saved.` to the console and quit.

After a successful first-time setup, simply run `audioleaf &` to run the program in the background. You can also set up a systemd unit to launch audioleaf (for example) on PC startup.

## Configuration
All configuration is done through the `audioleaf.toml` file, whose location is `$HOME/.config/audioleaf`.
