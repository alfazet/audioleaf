use palette::Hwb;
use rustfft::{num_complex::Complex32, FftPlanner};

// k - the bigger, the steeper the curve
fn sigmoid(x: f32, k: f32) -> f32 {
    let e_term = (-k * x).exp();

    (1.0 - e_term) / (1.0 + e_term)
}

pub fn process(time_samples: Vec<f32>, k: f32) -> Vec<f32> {
    let n_samples = time_samples.len();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n_samples);

    let mut complex_time_samples = time_samples
        .into_iter()
        .map(|x| Complex32 { re: x, im: 0.0 })
        .collect::<Vec<_>>();
    fft.process(&mut complex_time_samples);
    let root_n = (n_samples as f32).sqrt();

    // normalization and sigmoid, so that the amplitudes end up in the range [0, 1]
    complex_time_samples
        .into_iter()
        .take(n_samples / 2)
        .map(|z| sigmoid(z.norm() / root_n, k))
        .collect::<Vec<_>>()
}

pub fn visualize(
    spectrum: Vec<f32>,
    min_freq: u32,
    max_freq: u32,
    base_colors: &[u16],
    hz_per_bin: u32,
) -> Vec<Hwb> {
    let n_panels = base_colors.len();
    let (min_freq, max_freq) = (min_freq as f32, max_freq as f32);
    let multiplier = (max_freq / min_freq).powf(1.0 / n_panels as f32);
    let (mut intervals, mut cutoff) = (Vec::new(), min_freq);
    for _ in 0..n_panels {
        cutoff *= multiplier;
        intervals.push(cutoff.round().min(max_freq) as u32);
    }
    let (n_bins, mut cur_interval, mut cur_max) = (spectrum.len(), 0, 0.0_f32);
    let mut colors = Vec::new();
    for (i, ampl) in spectrum.into_iter().enumerate() {
        let cur_freq = (i as u32) * hz_per_bin + hz_per_bin / 2;
        if cur_freq > intervals[cur_interval] || i == n_bins - 1 {
            colors.push(Hwb::new(
                base_colors[cur_interval] as f32,
                1.0 - cur_max,
                0.0,
            ));
            cur_max = 0.0_f32;
            cur_interval += 1;
        }
        if cur_freq > *intervals.last().unwrap() {
            break;
        }
        cur_max = cur_max.max(ampl);
    }
    assert!(colors.len() == n_panels);

    colors
}
