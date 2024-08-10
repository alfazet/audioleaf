use num::complex::Complex;
use std::f32::consts::PI;

const I: Complex<f32> = Complex { re: 0.0, im: 1.0 };

// based on rosettacode
pub fn fft(samples: &[Complex<f32>]) -> Vec<Complex<f32>> {
    fn rec(buf_a: &mut [Complex<f32>], buf_b: &mut [Complex<f32>], n: usize, step: usize) {
        if step >= n {
            return;
        }

        rec(buf_b, buf_a, n, step * 2);
        rec(&mut buf_b[step..], &mut buf_a[step..], n, step * 2);
        let (left, right) = buf_a.split_at_mut(n / 2);

        for k in (0..n).step_by(step * 2) {
            let t = (-I * PI * (k as f32) / (n as f32)).exp() * buf_b[k + step];
            left[k / 2] = buf_b[k] + t;
            right[k / 2] = buf_b[k] - t;
        }
    }

    let n = samples.len();
    let n_padded = n.next_power_of_two();
    let mut buf_a = samples.to_vec();
    buf_a.append(&mut vec![Complex { re: 0.0, im: 0.0 }; n_padded - n]);
    let mut buf_b = buf_a.clone();
    rec(&mut buf_a, &mut buf_b, n, 1);

    buf_a
}
