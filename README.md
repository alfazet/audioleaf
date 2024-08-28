# Audioleaf

An audio visualizer for Nanoleaf Canvas

## Installation

Install from cargo with `cargo install audioleaf`. Make sure that the directory with cargo binaries (by default `$HOME\.cargo\bin`) is added to your `$PATH`.

For users of Arch-based distros, audioleaf is also available as a [package in the AUR](https://aur.archlinux.org/packages/audioleaf). You can install it with your AUR helper of choice, for example with yay: `yay -S audioleaf`.

## Configuration
All configuration of audioleaf is done through the `audioleaf.toml` file, located in `$HOME/.config/audioleaf`. All the options are described below:

* `audio_device`: The audio input device that will be the source of audio data for the visualizer.
* `min/max_freq`: The minimum/maximum frequency (in Hz) to be included in the visualization. 
* `default_gain`: A non-negative real number, the bigger it is the more the audio is boosted before being visualized. While in audioleaf you can decrease and increase gain with the <kbd>-</kbd> and <kbd>=</kbd> keys. This settings doesn't affect your listening volume.
* `hues`: A list of hues to be used in the visualizer's color palette, specified as angles between 0 and 360 degrees on the standard [color wheel](https://developer.mozilla.org/en-US/blog/learn-css-hues-colors-hsl/color-wheel.svg).
* `nl_config.primary_axis`: The primary coordinate by which the panels will be sorted. Possible values are `"x"` (left → right) and `
"y"` (bottom → top).
* `nl_config.sort_primary/secondary`: The direction in which the panels will be sorted on the primary/secondary axis. Possible values are `"asc"` (ascending) and `"desc"` (descending).
* `nl_config.active_panels`: A list of numbers of panels that should be lit up during visualization. These numbers relate to the sorting method mentioned earlier. For example, if you sorted your panels first by Y ascending, then by X descending, then the first panel will be in the lower right-hand corner of your setup and the last one will be in the upper left-hand corner.
* `nl_config.token_file_path`: The path where audioleaf will look for the file containing the Nanoleaf authentication token.
* `nl_config.ip`: The local IP address of your Nanoleaf device.
* `nl_config.port`: The port on your host to which the UDP socket handling the connection to the panels will be bound.

## Usage

>
IMPORTANT: When running audioleaf for the first time, do the following:
* Press and hold your Nanoleaf control panels's power button for about 5 seconds, until the icons start sequentially flashing.
* *While* the icons are flashing, run `audioleaf --ip <local ip address of your Nanoleaf device>` (for example `audioleaf --ip 192.168.0.101`). You can find that IP address by logging in to your WiFi router's admin panel and navigating to a "DHCP clients" list. Your Nanoleaf will have a MAC address starting with `00:55:DA:5`. Simply copy its assigned IP address from there.
* If everything went correctly, audioleaf will notify you about it and quit. Check your `~/.config` directory - a directory `audioleaf` with two files in it (`audioleaf.toml` - the config file and `nltoken` - containing the Nanoleaf auth token) will be there. 

Make sure to take a look at the config file and edit the options to match your Nanoleaf setup - the default settings are suited to my own panels and will very likely not work well with yours.

After a successful first-time setup, simply run `audioleaf` to launch the program. To see available options add the `--help` flag. Press <kbd>Shift</kbd> + <kbd>Q</kbd> to quit.

## Troubleshooting

Check that your audio input device for audioleaf is set correctly, especially if you have a microphone connected (since then the default input will most likely be your mic, and you probably want to visualize music, not your voice).
