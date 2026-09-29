//! Filters a flight log's channels are cleaned with before a reading is taken from them.

/// The running median of `values`: each sample replaced by the median of the samples within
/// `half` places of it, the window cut short at either end of the record. `NaN`s are skipped; a
/// window with no finite sample gives `NaN`. An even count takes the mean of the middle two.
///
/// This is the standard median filter, the Hampel filter at threshold `t = 0` (R. K. Pearson et
/// al., *The Class of Generalized Hampel Filters*, EUSIPCO 2015, §1, eqs. 1 and 2, with `K =
/// half`). It removes any excursion narrower than `half + 1` samples, such as the pressure pulse an
/// ejection charge punches into a barometric altitude, and leaves a monotonic run exactly as it
/// was. At a smooth peak it reads low, never high: by the fall over `⌈half/2⌉` samples from the
/// highest sample, as `half + 1` of the window's samples lie that close to it. Counting the half
/// sample by which the true peak can miss a sample too, a trace sampled every `Δt` whose downward
/// acceleration near its peak is at most `a` reads at most `a (half·Δt)² / 2` below its true peak,
/// for `half ≥ 2`.
pub fn running_median(values: &[f64], half: usize) -> Vec<f64> {
    let mut window = Vec::with_capacity(2 * half + 1);
    (0..values.len())
        .map(|index| {
            let start = index.saturating_sub(half);
            let end = (index + half + 1).min(values.len());
            window.clear();
            window.extend(values[start..end].iter().copied().filter(|v| v.is_finite()));
            median(&mut window).unwrap_or(f64::NAN)
        })
        .collect()
}

/// The Hampel filter: each sample more than `threshold` robust standard deviations from its
/// window's median replaced by that median, the scale being 1.4826 times the window's median
/// absolute deviation (R. K. Pearson et al., *The Class of Generalized Hampel Filters*, EUSIPCO
/// 2015, §1, eqs. 1 and 2, with `K = half` and `t = threshold`). Windows are cut short at the ends
/// and skip `NaN`s, as in [`running_median`], which is this filter at `threshold = 0`.
///
/// hpr reads heights after [`running_median`] instead (see
/// [`MEDIAN_WINDOW_S`](crate::readings::MEDIAN_WINDOW_S)); this is here to show why: a pulse
/// flanked by a dip, as an ejection charge's is, widens its own window's spread until the filter
/// keeps it.
pub fn hampel(values: &[f64], half: usize, threshold: f64) -> Vec<f64> {
    let mut window = Vec::with_capacity(2 * half + 1);
    (0..values.len())
        .map(|index| {
            let value = values[index];
            let start = index.saturating_sub(half);
            let end = (index + half + 1).min(values.len());
            window.clear();
            window.extend(values[start..end].iter().copied().filter(|v| v.is_finite()));
            let Some(middle) = median(&mut window) else {
                return value;
            };
            for sample in &mut window {
                *sample = (*sample - middle).abs();
            }
            let scale = 1.4826 * median(&mut window).unwrap_or(0.0);
            if (value - middle).abs() > threshold * scale {
                middle
            } else {
                value
            }
        })
        .collect()
}

/// The median of `values`, reordering them; `None` if there are none. An even count takes the
/// mean of the middle two. The values must not be `NaN`.
pub(crate) fn median(values: &mut [f64]) -> Option<f64> {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    match values.len() {
        0 => None,
        len if len % 2 == 1 => Some(values[middle]),
        _ => Some(0.5 * (values[middle - 1] + values[middle])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spike of up to `half` samples goes; a step, and a monotonic run, stay as they were.
    #[test]
    fn spikes_go_and_monotonic_runs_stay() {
        let spiked = [0.0, 0.0, 0.0, 0.0, 9.0, 8.0, 9.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(running_median(&spiked, 3), [0.0; 11]);
        // Where the window is cut short at an end, fewer samples outvote the spike.
        let early = [0.0, 0.0, 0.0, 9.0, 8.0, 9.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(running_median(&early, 3)[2], 4.0);
        let step = [0.0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0, 5.0];
        assert_eq!(running_median(&step, 3), step);
        let ramp: Vec<f64> = (0..20).map(|i| f64::from(i) * 1.5).collect();
        assert_eq!(running_median(&ramp, 3)[3..17], ramp[3..17]);
        assert!(running_median(&[f64::NAN, f64::NAN], 1)[0].is_nan());
        assert_eq!(running_median(&[1.0, f64::NAN, 3.0], 1), [1.0, 2.0, 3.0]);
    }

    /// The bound at a peak: a parabola `−a t²/2` sampled at `Δt` reads at most `a (half·Δt)²/2`
    /// low, and the peak sample's median is exactly the fall over `⌈half/2⌉` samples.
    #[test]
    fn a_peak_reads_low_by_at_most_the_fall_over_the_half_window() {
        let (a, dt, half) = (9.806_65, 0.05, 3_usize);
        let trace: Vec<f64> = (-20..=20)
            .map(|i| -0.5 * a * (f64::from(i) * dt).powi(2))
            .collect();
        let peak = running_median(&trace, half)[20];
        let bound = 0.5 * a * (3.0 * dt).powi(2);
        assert!(peak <= 0.0 && -peak <= bound, "{peak} against {bound}");
        assert_eq!(peak, trace[22]);
    }

    /// At threshold zero the Hampel filter is the running median, as Pearson et al. note.
    #[test]
    fn hampel_at_zero_is_the_running_median() {
        let values = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0, 5.0, 3.0, 5.0];
        for half in 1..4 {
            assert_eq!(hampel(&values, half, 0.0), running_median(&values, half));
        }
        // At threshold 4, a lone spike on a ramp goes, replaced by its window's median, and the
        // ramp stays.
        let ramp: Vec<f64> = (0..11).map(f64::from).collect();
        let mut spiked = ramp.clone();
        spiked[5] = 50.0;
        let mut expected = ramp;
        expected[5] = 6.0;
        assert_eq!(hampel(&spiked, 3, 4.0), expected);
    }

    proptest::proptest! {
        /// Each output lies between its window's least and greatest finite input.
        #[test]
        fn outputs_lie_within_their_windows(
            values in proptest::collection::vec(-1e6..1e6_f64, 0..60),
            half in 0_usize..6,
        ) {
            let out = running_median(&values, half);
            proptest::prop_assert_eq!(out.len(), values.len());
            for (index, value) in out.iter().enumerate() {
                let window = &values[index.saturating_sub(half)..(index + half + 1).min(values.len())];
                let low = window.iter().copied().fold(f64::INFINITY, f64::min);
                let high = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                proptest::prop_assert!(*value >= low && *value <= high);
            }
        }
    }
}
