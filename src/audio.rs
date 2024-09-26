use num::complex::Complex;
use palette::Hwb;

const PI: f32 = std::f32::consts::PI;
const I: Complex<f32> = Complex { re: 0.0, im: 1.0 };

fn fft(x: &mut [Complex<f32>], y: &mut [Complex<f32>], n: usize, step: usize) {
    if n == 1 {
        y[0] = x[0];
        return;
    }
    fft(x, y, n / 2, step * 2);
    fft(&mut x[step..], &mut y[(n / 2)..], n / 2, step * 2);
    for k in 0..(n / 2) {
        let t = (-2.0 * I * PI * (k as f32) / (n as f32)).exp() * y[k + n / 2];
        let temp = y[k];
        y[k] = temp + t;
        y[k + n / 2] = temp - t;
    }
}

pub fn process(samples: Vec<f32>, k: f32) -> Vec<f32> {
    let mut n = samples.len();
    let mut complex_samples = samples
        .into_iter()
        .map(|x| Complex::new(x, 0.0))
        .collect::<Vec<_>>();
    complex_samples.append(&mut vec![Complex::new(0.0, 0.0); n.next_power_of_two() - n]);
    n = complex_samples.len();
    let mut transformed_samples = complex_samples.clone();
    fft(&mut complex_samples, &mut transformed_samples, n, 1);

    // normalize and apply a sigmoid-type function (x / sqrt(1 + x^2)) so that amplitudes become numbers between 0 and 1
    let root_n = (n as f32).sqrt();
    transformed_samples
        .into_iter()
        .take(n / 2)
        .map(|z| {
            let x = k * z.norm() / root_n;
            x / (1.0 + x * x).sqrt()
        })
        .collect::<Vec<_>>()
}

pub fn update_colors(
    colors: &mut [Hwb],
    spectrum: Vec<f32>,
    min_freq: u32,
    max_freq: u32,
    hz_per_bin: u32,
) {
    let n_panels = colors.len();
    let (min_freq, max_freq) = (min_freq as f32, max_freq as f32);
    let multiplier = (max_freq / min_freq).powf(1.0 / n_panels as f32);
    let (mut intervals, mut cutoff) = (Vec::new(), min_freq);
    for _ in 0..n_panels {
        cutoff *= multiplier;
        intervals.push(cutoff.round().min(max_freq) as u32);
    }
    let (n_bins, mut cur_interval, mut cur_max) = (spectrum.len(), 0, 0.0);
    for (i, ampl) in spectrum.into_iter().enumerate() {
        let cur_freq = (i as u32) * hz_per_bin + hz_per_bin / 2;
        if cur_freq > intervals[cur_interval] || i == n_bins - 1 {
            let cur_whiteness = colors[cur_interval].whiteness;
            let rate_func = |x: f32| -> f32 { 0.75 * (0.5 * (PI * x)).sin() };
            if cur_max > 1.0 - cur_whiteness {
                // louder -> subtract whiteness
                colors[cur_interval].whiteness =
                    (cur_whiteness - rate_func(cur_max - 1.0 + cur_whiteness)).max(0.0);
            } else {
                // quieter -> add whiteness
                colors[cur_interval].whiteness =
                    (cur_whiteness + rate_func(1.0 - cur_whiteness - cur_max)).min(1.0);
            }
            cur_max = 0.0;
            cur_interval += 1;
        }
        if cur_freq > *intervals.last().unwrap() {
            break;
        }
        cur_max = cur_max.max(ampl);
    }
}
