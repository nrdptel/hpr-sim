//! Seeded pseudorandom numbers for turbulence, dispersions and Monte Carlo.
//!
//! [`SeededRng`] is **xoshiro256++**, seeded from one `u64` through **SplitMix64**, exactly as the
//! authors' reference code recommends:
//!
//! - D. Blackman and S. Vigna, "Scrambled linear pseudorandom number generators", *ACM Trans.
//!   Math. Softw.* 47(4), article 36 (2021), <https://doi.org/10.1145/3460772>; reference C code
//!   (public domain) at <https://prng.di.unimi.it/>.
//!
//! The algorithm is part of the result: the same seed gives the same integer stream on every
//! platform and in every release. Changing the generator, the seeding or the order of draws
//! changes every seeded result, so it needs an ADR.
//!
//! Normal deviates use the polar method of G. Marsaglia and T. A. Bray, "A convenient method for
//! generating normal variables", *SIAM Review* 6(3), 260–264 (1964): draw `u, v` uniform on
//! `(−1, 1)` until `0 < s = u² + v² < 1`, then `u·√(−2 ln s / s)` and `v·√(−2 ln s / s)` are two
//! independent standard normal deviates. The second one is kept for the next call. The method
//! needs only `ln` and `sqrt`, so normal deviates are bit-identical on one platform, and across
//! platforms wherever their math libraries' `ln` agree.

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// The SplitMix64 increment, `⌊2⁶⁴/φ⌋` (odd).
const SPLITMIX64_GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;

/// One SplitMix64 step: advances `state` and returns the next output (Steele, Lea and Flood,
/// "Fast splittable pseudorandom number generators", OOPSLA 2014; the constants are those of the
/// public-domain reference at <https://prng.di.unimi.it/splitmix64.c>).
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX64_GAMMA);
    mix64(*state)
}

/// SplitMix64's output function: a bijection on 64-bit words that scrambles every input bit into
/// every output bit.
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A seeded xoshiro256++ generator with a standard normal sampler.
///
/// It serializes as its 256-bit state, written as four hexadecimal strings (`"0x…"`, because JSON
/// readers such as JavaScript's lose integers above 2⁵³), and the spare normal deviate, so a run
/// can be checkpointed and resumed bit for bit. The all-zero state is rejected: xoshiro would stay
/// at zero forever.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SeededRngData", into = "SeededRngData")]
pub struct SeededRng {
    state: [u64; 4],
    spare_normal: Option<f64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SeededRngData {
    state: [String; 4],
    #[serde(default)]
    spare_normal: Option<f64>,
}

/// Parses one state word written as `0x` and 1 to 16 hexadecimal digits.
fn parse_state_word(text: &str) -> Result<u64, CoreError> {
    text.strip_prefix("0x")
        .filter(|digits| {
            !digits.is_empty()
                && digits.len() <= 16
                && digits.bytes().all(|b| b.is_ascii_hexdigit())
        })
        .and_then(|digits| u64::from_str_radix(digits, 16).ok())
        .ok_or(CoreError::InvalidRandomState)
}

impl TryFrom<SeededRngData> for SeededRng {
    type Error = CoreError;

    fn try_from(data: SeededRngData) -> Result<Self, CoreError> {
        let mut state = [0; 4];
        for (word, text) in state.iter_mut().zip(&data.state) {
            *word = parse_state_word(text)?;
        }
        if state == [0; 4] {
            return Err(CoreError::InvalidRandomState);
        }
        if let Some(spare) = data.spare_normal
            && !spare.is_finite()
        {
            return Err(CoreError::Domain {
                what: "spare normal deviate",
                value: spare,
            });
        }
        Ok(SeededRng {
            state,
            spare_normal: data.spare_normal,
        })
    }
}

impl From<SeededRng> for SeededRngData {
    fn from(rng: SeededRng) -> Self {
        SeededRngData {
            state: rng.state.map(|word| format!("{word:#018x}")),
            spare_normal: rng.spare_normal,
        }
    }
}

impl SeededRng {
    /// Seeds the generator: the four state words are the first four SplitMix64 outputs starting
    /// from `seed`. SplitMix64 never gives four zero words in a row, so every seed is valid.
    pub fn seed_from_u64(seed: u64) -> Self {
        let mut sm = seed;
        let state = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        SeededRng {
            state,
            spare_normal: None,
        }
    }

    /// A generator for one stream of a seeded run, keyed by `keys`: a Monte Carlo sample's index
    /// and the variable it draws, say. The stream depends only on `seed` and `keys`, so a sample
    /// draws the same numbers whatever else the run draws, however many samples it has and
    /// however they are shared among threads: the counter-based idea of J. K. Salmon, M. A.
    /// Moraes, R. O. Dror and D. E. Shaw, "Parallel random numbers: as easy as 1, 2, 3", *Proc.
    /// SC11* (2011), <https://doi.org/10.1145/2063384.2063405>.
    ///
    /// The key folds into one word with SplitMix64's output function `mix`, a bijection on 64-bit
    /// words: `h₀ = seed`, `hᵢ = mix(hᵢ₋₁ ⊕ mix(kᵢ + γ))` with `γ` SplitMix64's increment, and the
    /// generator is [`SeededRng::seed_from_u64`]`(hₙ)`. Two different keys give the same `hₙ` only
    /// by a collision of 64-bit hashes, about one chance in 2⁶⁴ for a pair of streams; with no
    /// keys the stream is `seed_from_u64(seed)`'s.
    pub fn for_stream(seed: u64, keys: &[u64]) -> Self {
        let hash = keys.iter().fold(seed, |h, &key| {
            mix64(h ^ mix64(key.wrapping_add(SPLITMIX64_GAMMA)))
        });
        Self::seed_from_u64(hash)
    }

    /// The next 64 random bits (the xoshiro256++ output function and state transition).
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.state;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// A uniform deviate on `[0, 1)`: the top 53 bits of [`SeededRng::next_u64`] times `2⁻⁵³`,
    /// so every value is a multiple of `2⁻⁵³`.
    pub fn uniform(&mut self) -> f64 {
        // 2⁻⁵³ as an exact f64 literal.
        const SCALE: f64 = 1.0 / 9_007_199_254_740_992.0;
        // Cast: a 53-bit integer converts to f64 exactly.
        let top = (self.next_u64() >> 11) as f64;
        top * SCALE
    }

    /// A standard normal deviate (mean 0, variance 1) by the Marsaglia–Bray polar method.
    ///
    /// Each accepted pair gives two deviates; the second is returned by the next call. On average
    /// a pair costs `4/π ≈ 1.27` attempts, two uniforms each.
    pub fn standard_normal(&mut self) -> f64 {
        if let Some(spare) = self.spare_normal.take() {
            return spare;
        }
        loop {
            let u = 2.0 * self.uniform() - 1.0;
            let v = 2.0 * self.uniform() - 1.0;
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                let factor = (-2.0 * s.ln() / s).sqrt();
                self.spare_normal = Some(v * factor);
                return u * factor;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand_core::{Rng, SeedableRng};
    use rand_xoshiro::{SplitMix64, Xoshiro256PlusPlus};

    use super::*;

    /// The generator and its seeding match the rust-random `rand_xoshiro` crate, an independent
    /// implementation of the same public-domain reference code.
    #[test]
    fn stream_matches_the_rand_xoshiro_implementation() {
        for seed in [0, 1, 42, 0x0123_4567_89ab_cdef, u64::MAX] {
            let mut ours = SeededRng::seed_from_u64(seed);
            let mut theirs = Xoshiro256PlusPlus::seed_from_u64(seed);
            for _ in 0..10_000 {
                assert_eq!(ours.next_u64(), theirs.next_u64(), "seed {seed}");
            }
        }
    }

    #[test]
    fn seeding_is_four_splitmix64_outputs() {
        let mut theirs = SplitMix64::seed_from_u64(7);
        let mut sm = 7;
        for _ in 0..100 {
            assert_eq!(splitmix64(&mut sm), theirs.next_u64());
        }
    }

    #[test]
    fn a_stream_depends_only_on_its_seed_and_keys() {
        // No keys: the seed's own stream.
        assert_eq!(SeededRng::for_stream(42, &[]), SeededRng::seed_from_u64(42));
        // The fold, written out: one key is mix(seed ^ mix(key + γ)), seeded by SplitMix64.
        let mut sm = 7_u64;
        let inner = splitmix64(&mut sm);
        let mut outer = (42 ^ inner).wrapping_sub(SPLITMIX64_GAMMA);
        assert_eq!(
            SeededRng::for_stream(42, &[7]),
            SeededRng::seed_from_u64(splitmix64(&mut outer))
        );
        // Repeatable, and different for a different seed, key, key order or key count.
        let mut first = SeededRng::for_stream(1, &[3, 5]);
        let mut again = SeededRng::for_stream(1, &[3, 5]);
        let a: Vec<u64> = (0..4).map(|_| first.next_u64()).collect();
        let b: Vec<u64> = (0..4).map(|_| again.next_u64()).collect();
        assert_eq!(a, b);
        let others = [
            SeededRng::for_stream(2, &[3, 5]),
            SeededRng::for_stream(1, &[3, 6]),
            SeededRng::for_stream(1, &[5, 3]),
            SeededRng::for_stream(1, &[3]),
            SeededRng::for_stream(1, &[3, 5, 0]),
        ];
        for mut other in others {
            assert_ne!(other.next_u64(), a[0]);
        }
    }

    #[test]
    fn neighbouring_streams_are_uncorrelated() {
        // The first uniform of 20,000 streams keyed 0, 1, 2…: the mean and the lag-one
        // correlation of independent uniforms, within four standard errors.
        let n = 20_000_u64;
        let u: Vec<f64> = (0..n)
            .map(|k| SeededRng::for_stream(2026, &[k]).uniform())
            .collect();
        let count = n as f64;
        let mean = u.iter().sum::<f64>() / count;
        assert!(
            (mean - 0.5).abs() < 4.0 * (1.0 / 12.0 / count).sqrt(),
            "{mean}"
        );
        let lag: f64 = u
            .windows(2)
            .map(|w| (w[0] - 0.5) * (w[1] - 0.5))
            .sum::<f64>()
            / (count - 1.0)
            * 12.0;
        assert!(lag.abs() < 4.0 / (count - 1.0).sqrt(), "{lag}");
    }

    #[test]
    fn uniform_is_in_the_half_open_unit_interval() {
        let mut rng = SeededRng::seed_from_u64(3);
        let n = 1_000_000;
        let mut sum = 0.0;
        for _ in 0..n {
            let x = rng.uniform();
            assert!((0.0..1.0).contains(&x));
            sum += x;
        }
        // Mean of U(0,1) is 1/2 with standard error √(1/12/n) ≈ 2.9e-4; allow 5 standard errors.
        let mean = sum / f64::from(n);
        assert!((mean - 0.5).abs() < 5.0 * (1.0 / 12.0 / f64::from(n)).sqrt());
    }

    /// Mean, variance, and the empirical CDF at −2, −1, 0, 1 and 2 standard deviations, each
    /// within 5 standard errors for 10⁶ draws. Φ values from M. Abramowitz and I. A. Stegun,
    /// *Handbook of Mathematical Functions*, Table 26.1.
    #[test]
    fn standard_normal_has_the_normal_moments_and_cdf() {
        let mut rng = SeededRng::seed_from_u64(2026);
        let n = 1_000_000;
        let points = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let phi = [
            0.022_750_131_948_179,
            0.158_655_253_931_457,
            0.5,
            0.841_344_746_068_543,
            0.977_249_868_051_821,
        ];
        let mut below = [0_u32; 5];
        let (mut sum, mut sum_sq, mut sum_4) = (0.0, 0.0, 0.0);
        for _ in 0..n {
            let x = rng.standard_normal();
            sum += x;
            sum_sq += x * x;
            sum_4 += x * x * x * x;
            for (count, &point) in below.iter_mut().zip(&points) {
                if x <= point {
                    *count += 1;
                }
            }
        }
        let nf = f64::from(n);
        let mean = sum / nf;
        let variance = sum_sq / nf - mean * mean;
        assert!(mean.abs() < 5.0 / nf.sqrt(), "mean {mean}");
        // Var of the sample variance of N(0,1) is 2/n.
        assert!(
            (variance - 1.0).abs() < 5.0 * (2.0 / nf).sqrt(),
            "variance {variance}"
        );
        // E[x⁴] = 3 with Var = (105 − 9)/n.
        assert!((sum_4 / nf - 3.0).abs() < 5.0 * (96.0 / nf).sqrt());
        for ((&count, &p), &point) in below.iter().zip(&phi).zip(&points) {
            let fraction = f64::from(count) / nf;
            let standard_error = (p * (1.0 - p) / nf).sqrt();
            assert!(
                (fraction - p).abs() < 5.0 * standard_error,
                "CDF at {point}: {fraction} vs {p}"
            );
        }
    }

    #[test]
    fn same_seed_same_normals_and_serde_resumes_the_stream() {
        let mut a = SeededRng::seed_from_u64(99);
        let mut b = SeededRng::seed_from_u64(99);
        for _ in 0..1001 {
            assert_eq!(a.standard_normal().to_bits(), b.standard_normal().to_bits());
        }
        // Mid-pair: `a` holds a spare deviate, which the checkpoint must carry.
        assert!(a.spare_normal.is_some());
        let json = serde_json::to_string(&a).unwrap();
        let mut resumed: SeededRng = serde_json::from_str(&json).unwrap();
        for _ in 0..1000 {
            assert_eq!(
                a.standard_normal().to_bits(),
                resumed.standard_normal().to_bits()
            );
        }
    }

    #[test]
    fn state_serializes_as_hex_words_and_bad_states_are_rejected() {
        let rng = SeededRng::seed_from_u64(1);
        let value = serde_json::to_value(&rng).unwrap();
        let words = value["state"].as_array().unwrap();
        assert_eq!(words.len(), 4);
        for (word, &expected) in words.iter().zip(&rng.state) {
            let text = word.as_str().unwrap();
            assert_eq!(text.len(), 18);
            assert_eq!(parse_state_word(text).unwrap(), expected);
        }
        let zero = r#"{"state":["0x0","0x0","0x0","0x00"],"spare_normal":null}"#;
        assert_eq!(
            serde_json::from_str::<SeededRng>(zero)
                .unwrap_err()
                .to_string(),
            CoreError::InvalidRandomState.to_string()
        );
        for bad in [
            r#"{"state":[1,2,3,4]}"#,
            r#"{"state":["1","0x2","0x3","0x4"]}"#,
            r#"{"state":["0x","0x2","0x3","0x4"]}"#,
            r#"{"state":["0x10000000000000000","0x2","0x3","0x4"]}"#,
            r#"{"state":["0xg","0x2","0x3","0x4"]}"#,
            r#"{"state":["0x+1","0x2","0x3","0x4"]}"#,
        ] {
            assert!(serde_json::from_str::<SeededRng>(bad).is_err(), "{bad}");
        }
        let short = r#"{"state":["0x1","0x2","0x3","0x4"]}"#;
        assert_eq!(
            serde_json::from_str::<SeededRng>(short).unwrap().state,
            [1, 2, 3, 4]
        );
    }
}
