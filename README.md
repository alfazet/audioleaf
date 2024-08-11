# Audioleaf

A music visualizer for Nanoleaf Canvas. Works best with MPD (the Music Player Daemon) on Linux.

## Installation

Install from cargo using `cargo install audioleaf`. Make sure that the directory with cargo binaries (by default `$HOME\.cargo\bin`) is added to your PATH.

## Usage

>
IMPORTANT: When running audioleaf for the first time, do the following:
* Press and hold your Nanoleaf control panels's power button for about 5 seconds, until the icons start sequentially flashing.
* *While* the icons are flashing, run `audioleaf <local ip address of your Nanoleaf device>` (for example `audioleaf 192.168.0.101`). You can find that IP address by logging in to your WiFi router's page and navigating to the DHCP clients list. Your Nanoleaf will have a MAC address starting with `00:55:DA:5`. Simply copy the assigned IP address from there.
* If everything went correctly, audioleaf will notify you about it and quit.

After a successful first-time setup, simply run `audioleaf &` to run the program in the background. You can also set up a systemd unit to launch audioleaf (for example) on PC startup.

## Configuration
All configuration is done through the `audioleaf.toml` file, located in `$HOME/.config/audioleaf`. Options and their data types are described below:

* `fifo_path`: The path of the .fifo file, which is the source of PCM data for the visualizer.
* `sample_rate`: The sample rate of your audio files.
* `n_samples`: How many samples will be taken by the visualizer at one time.
* `max_volume_level` : The highest volume level "representable" on the visualizer. Every volume level greater than or equal to `max_volume_level` will be mapped to the brightest color for a given panel.
    * The aforementioned "volume level" is the average over natural logs of the magnitudes of frequencies included in a certain time interval. It doesn't directly correspond to any physical unit like dB. In general, this number doesn't exceed 16. Also note that it *doesn't* depend on the volume of your audio output device. 
* `brightness_range`: A real value between 0 and 100, specifying how much the color brightness will change in response to changes in the volume level.
* `nl_config.ip`: The local IP address of your Nanoleaf device.
* `port`: The port to which the UDP socket will bind (on your host).
* `token_file_path`: The path where audioleaf will save the Nanoleaf auth token.
* `primary_axis`: The primary coordinate by which the panels will be sorted. Possible values are `x` and `y`.
* `sort_primary`: The direction in which the panels will be sorted on the primary axis. Possible values are `asc` (ascending) and `desc` (descending).
* `sort_secondary`: Ditto, secondary axis.
* `active_panels`: A list of numbers of panels that should be lit up during visualization. Keep in mind that those *aren't* panel IDs, the numbers here relate to the sorting method mentioned earlier. For example, if you sorted your panels first by Y ascending, then by X descending, then panel #1 will be the one in the lower right-hand corner of your setup.
* `trans_time`: An integer value specifying how much time (in multiples of 100ms) a transition from one color to the next should take. Note that setting this value too high might cause transitions to be interrupted in the middle. In general it should be 1 or 2. 
* `freq_ranges`: A table of pairs `(cutoff, color)` which specify how to split the spectrum of frequencies. Take for example the following snippet:
```toml
[[freq_ranges]]
cutoff = 10000
color = "#ff0000"

[[freq_ranges]]
cutoff = 20000
color = "#0000ff"
```

In this configuration, frequencies from 0 to 10000 Hz will be visualized in a shade of red, and those between 10000 and 20000 Hz in a shade of blue. Note that the cutoff frequencies are in hertz and colors are given as #rrggbb.

The last cutoff has to be at least equal to half of the `sample_rate`. 

The number of frequency ranges must be less than or equal to the number of active panels (see `active_panels`).
