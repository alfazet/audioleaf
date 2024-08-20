# Audioleaf

A music visualizer for Nanoleaf Canvas. Works best with MPD (the Music Player Daemon) on Linux.

## Installation

Install from cargo with `cargo install audioleaf`. Make sure that the directory with cargo binaries (by default `$HOME\.cargo\bin`) is added to your PATH.

For users of Arch-based distros, audioleaf is also available as a [package in the AUR](https://aur.archlinux.org/packages/audioleaf). You can install it with your AUR helper, for example with yay: `yay -S audioleaf`.

## Usage

>
IMPORTANT: When running audioleaf for the first time, do the following:
* Press and hold your Nanoleaf control panels's power button for about 5 seconds, until the icons start sequentially flashing.
* *While* the icons are flashing, run `audioleaf <local ip address of your Nanoleaf device>` (for example `audioleaf 192.168.0.101`). You can find that IP address by logging in to your WiFi router's admin panel and navigating to a "DHCP clients" list. Your Nanoleaf will have a MAC address starting with `00:55:DA:5`. Simply copy its assigned IP address from there.
* If everything went correctly, audioleaf will notify you about it and quit.

After a successful first-time setup, simply run `audioleaf` to launch the program (`audioleaf &` will run it in the background). You can also set up a systemd unit to (for example) launch audioleaf on startup.

This software is best paired with [MPD](https://www.musicpd.org/), which very conveniently provides a source of PCM data for visualization. To enable this source (a named pipe in UNIX terms), you need to add these lines to your MPD config file:
```
audio_output {
    type                    "fifo"
    name                    "my_fifo"
    path                    "/tmp/mpd.fifo"
    format                  "44100:16:2"
}
```
Note that the `path` you specify must be the same as the `path` variable in audioleaf's config file (see below).

## Configuration
All configuration of audioleaf is done through the `audioleaf.toml` file, located in `$HOME/.config/audioleaf`. Options and their data types are described here:

* `fifo_path`: The path of the .fifo file, which is the source of PCM data for the visualizer.
* `sample_rate`: The sample rate of your audio files. 
* `n_samples`: How many samples will be taken by the visualizer in one "batch".
* `max_volume_level` : The highest volume level "representable" on the visualizer. Every volume level greater than or equal to `max_volume_level` will be mapped to the brightest color for a given panel.
    * The aforementioned "volume level" is the average over logs of the magnitudes of frequencies included in a certain time interval. It doesn't directly correspond to any physical unit like dB. In general, this number won't exceed 16. Also note that it *doesn't* depend on the volume of your output device (the volume that you hear). 
* `brightness_range`: A real value between 0 and 100, specifying how much the color brightness will change in response to changes in the volume level. It should be set pretty high for a more "lively" effect.
* `nl_config.ip`: The local IP address of your Nanoleaf device.
* `port`: The port to which the UDP socket will bind (on your host).
* `token_file_path`: The path where audioleaf will save the Nanoleaf auth token.
* `primary_axis`: The primary coordinate by which the panels will be sorted. Possible values are `x` (left -> right) and `y` (bottom -> top).
* `sort_primary`: The direction in which the panels will be sorted on the primary axis. Possible values are `asc` (ascending) and `desc` (descending).
* `sort_secondary`: Ditto, secondary axis.
* `active_panels`: A list of numbers of panels that should be lit up during visualization. Keep in mind that those *aren't* panel IDs, the numbers here relate to the sorting method mentioned earlier. For example, if you sorted your panels first by Y ascending, then by X descending, then panel #1 will be the one in the lower right-hand corner of your setup.
* `trans_time`: An integer value specifying how much time (in multiples of 100ms) a transition from one color to the next should take. Note that setting this value too high might cause transitions to be interrupted in the middle. In general it should be 1 or 2. 
* `freq_ranges`: A table of pairs `(cutoff, color)` which specify how to split the spectrum of frequencies. This is best explained with an example:
```toml
[[freq_ranges]]
cutoff = 10000
color = "#ff0000"

[[freq_ranges]]
cutoff = 20000
color = "#0000ff"
```

In this configuration, frequencies from 0 to 10000 Hz will be visualized in a shade of red, and those between 10000 and 20000 Hz in a shade of blue. Note that the cutoff frequencies are specified in Hz and colors are given as #rrggbb hex codes.

The last frequency cutoff has to be at least equal to half of the `sample_rate`. 

The number of frequency ranges must be less than or equal to the number of active panels (see `active_panels`).

## Configuration tips
To achieve the best synchronization between music and Nanoleaf set `trans_time` to a value near `n_samples / sample_rate`. This way the amount of time it will take for the visualizer to process a single batch of samples will be roughly the same as the time it takes your Nanoleaf panels to transition from one color to another.

For instance: with a `sample_rate` of 48 kHz and `n_samples` set to 8192 we have `n_samples / sample_rate = 8192 / 48000 ≈ 0.17 (s)`. It means that we will get the best sync with a `trans_time` of 2 (2 * 100 ms, so 0.2 s).
