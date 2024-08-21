use crate::config::FreqRange;
use crate::fft;
use expanduser::expanduser;
use num::complex::Complex;
use palette::{FromColor, Hwb, Srgb};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

/// Return PCM samples from a .fifo (named pipe)
pub fn get_samples(fifo_path: &str, n_samples: usize) -> Result<Vec<i16>, Box<dyn Error>> {
    let fifo_path_expanded = expanduser(fifo_path)?.display().to_string();
    let mut fifo = match File::open(&fifo_path_expanded) {
        Ok(fifo) => fifo,
        Err(_) => {
            return Err(format!("Couldn't open file {}.", fifo_path_expanded).into());
        }
    };
    let mut buf = vec![0u8; n_samples * 4];
    fifo.read_exact(&mut buf)?;
    let mut samples = Vec::new();
    for i in (0..(n_samples * 4)).step_by(4) {
        let chan_l = i16::from_le_bytes([buf[i], buf[i + 1]]);
        let chan_r = i16::from_le_bytes([buf[i + 2], buf[i + 3]]);
        samples.push(chan_l.saturating_add(chan_r.saturating_sub(chan_l)) / 2);
    }

    Ok(samples)
}

/// Transform samples from time to frequency domain using FFT
pub fn freq_domain(samples: &[i16]) -> Vec<f32> {
    let complex_samples = samples
        .iter()
        .map(|x| Complex::from(*x as f32))
        .collect::<Vec<Complex<f32>>>();

    fft::fft(&complex_samples)
        .iter_mut()
        .take(samples.len() / 2) // the second half is a mirror image of the first
        .map(|x| {
            let norm = Complex::norm(*x);
            if norm.abs() < 1.0 {
                // in case of near silence we want 0, not a negative number
                0.0
            } else {
                norm.ln()
            }
        }) // log the values, so that they're less spread apart
        .collect::<Vec<f32>>()
}

/// Return n colors corresponding to the given frequency spectrum
pub fn visualise(
    spectrum: Vec<f32>,
    freq_ranges: &[FreqRange],
    n_panels: usize,
    sample_rate: usize,
    cur_base_level: f32,
    max_volume_level: f32,
    brightness_range: f32,
) -> Result<Vec<Hwb>, Box<dyn Error>> {
    if !(0.0..=100.0).contains(&brightness_range) {
        return Err("Brightness range must be a real number between 0 and 100.".into());
    }
    let last_freq_cutoff = match freq_ranges.last() {
        Some(last_cutoff) => last_cutoff.cutoff,
        None => {
            return Err("There has to be at least one specified frequency range.".into());
        }
    };
    if last_freq_cutoff < sample_rate / 2 {
        return Err(format!("The last frequency range needs to end at at least a half of the sample_rate ({} Hz in this case).", sample_rate / 2).into());
    }
    let mut res = Vec::new();
    let panels_per_range = n_panels / freq_ranges.len();
    if panels_per_range == 0 {
        return Err(
            "The number of panels can't be lower than the number of specified frequency ranges."
                .into(),
        );
    }
    let n_bins = spectrum.len();
    let hz_per_bin = (sample_rate / 2) / n_bins;
    let middle_offset = hz_per_bin / 2;
    // let mid_volume_level = max_volume_level.div_euclid(2.0);
    let (mut cur_range, mut bins_in_range, mut total_level) = (0, 0, 0.0);
    for (i, level) in spectrum.into_iter().enumerate() {
        if i * hz_per_bin + middle_offset > freq_ranges[cur_range].cutoff || i == n_bins - 1 {
            let color_rgb = freq_ranges[cur_range]
                .color
                .parse::<Srgb<u8>>()?
                .into_format::<f32>();
            let mut color_hwb = Hwb::from_color(color_rgb).into_format::<f32>();
            let avg_level = total_level / (bins_in_range as f32);
            let brightness_delta =
                (avg_level - cur_base_level) * (brightness_range / cur_base_level) / 100.0;
            if brightness_delta >= 0.0 {
                color_hwb.whiteness += brightness_delta;
            } else {
                color_hwb.blackness -= brightness_delta;
            }
            for _ in 0..panels_per_range {
                res.push(color_hwb);
            }
            cur_range += 1;
            total_level = 0.0;
            bins_in_range = 0;
        }
        total_level += level.min(max_volume_level);
        bins_in_range += 1;
    }

    Ok(res)
}
