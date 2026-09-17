//! Impulse classes: the letter in a motor's designation that bands its total impulse.
//!
//! Each class tops out at twice the one before it. The National Association of Rocketry's
//! *Standard Motor Codes* table gives the upper limits, from `1/8A` (0.3125 N·s) through `O`
//! (40 960 N·s), and its text states the bands as, for example, "5.01 to 10.0 N-sec" for `C`: a
//! class's **upper limit belongs to it**, and the lower limit belongs to the class below.
//!
//! ```text
//! upper(k) = 1.25 · 2^k N·s,   k = −2 for 1/8A, −1 for 1/4A, 0 for 1/2A, 1 for A, …, 15 for O
//! ```
//!
//! `1/8A` starts at zero. Letters past `O` (`P`, `Q`, …) continue the doubling, as research and
//! amateur rocketry use them; COTS motors stop at `O`. See `docs/physics/motor.md`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::MotorError;

/// The lowest class exponent, `1/8A`.
const MIN_EXPONENT: i8 = -2;
/// The highest class exponent, `Z`.
const MAX_EXPONENT: i8 = 26;

/// A motor's impulse class, such as `1/4A`, `C` or `M`.
///
/// Serializes as its label (`"1/2A"`, `"H"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImpulseClass {
    /// `k` in `upper(k) = 1.25 · 2^k N·s`: −2 for `1/8A`, 1 for `A`, 26 for `Z`.
    exponent: i8,
}

impl ImpulseClass {
    /// The class a total impulse falls in. Upper limits are inclusive: 2.5 N·s is an `A`, and
    /// anything above it up to 5 N·s is a `B`.
    ///
    /// # Errors
    ///
    /// [`MotorError::Domain`] when the impulse is not finite, not positive, or above the `Z` limit
    /// (1.25 · 2²⁶ N·s).
    pub fn from_total_impulse(total_impulse_ns: f64) -> Result<Self, MotorError> {
        if !(total_impulse_ns.is_finite() && total_impulse_ns > 0.0) {
            return Err(MotorError::Domain {
                what: "total impulse (N·s)",
                value: total_impulse_ns,
            });
        }
        // Powers of two times 1.25 are exact in f64, so the band edges compare exactly.
        (MIN_EXPONENT..=MAX_EXPONENT)
            .map(|exponent| Self { exponent })
            .find(|class| total_impulse_ns <= class.upper_limit_ns())
            .ok_or(MotorError::Domain {
                what: "total impulse (N·s)",
                value: total_impulse_ns,
            })
    }

    /// The largest total impulse in the class, N·s (inclusive).
    pub fn upper_limit_ns(self) -> f64 {
        1.25 * 2f64.powi(i32::from(self.exponent))
    }

    /// The largest total impulse of the class below, N·s (exclusive); zero for `1/8A`.
    pub fn lower_limit_ns(self) -> f64 {
        if self.exponent == MIN_EXPONENT {
            0.0
        } else {
            0.5 * self.upper_limit_ns()
        }
    }

    /// The class label: `1/8A`, `1/4A`, `1/2A`, then `A` to `Z`.
    pub fn label(self) -> String {
        match self.exponent {
            -2 => "1/8A".to_owned(),
            -1 => "1/4A".to_owned(),
            0 => "1/2A".to_owned(),
            k => {
                // `k` is in 1..=26 by construction, so the letter is in 'A'..='Z'.
                let offset = u8::try_from(k - 1).unwrap_or(0);
                char::from(b'A' + offset).to_string()
            }
        }
    }
}

impl fmt::Display for ImpulseClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

impl FromStr for ImpulseClass {
    type Err = MotorError;

    /// Parses a label (`1/2A`, `c`, `M`), case-insensitively.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let label = s.trim().to_ascii_uppercase();
        let exponent = match label.as_str() {
            "1/8A" => -2,
            "1/4A" => -1,
            "1/2A" => 0,
            _ => match label.as_bytes() {
                [letter @ b'A'..=b'Z'] => i8::try_from(letter - b'A').unwrap_or(0) + 1,
                _ => {
                    return Err(MotorError::Parse {
                        what: "impulse class",
                        text: s.to_owned(),
                    });
                }
            },
        };
        Ok(Self { exponent })
    }
}

impl TryFrom<String> for ImpulseClass {
    type Error = MotorError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<ImpulseClass> for String {
    fn from(class: ImpulseClass) -> Self {
        class.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nar_table_upper_limits() {
        // NAR, Standard Motor Codes, "Impulse Limit" column.
        let table = [
            ("1/8A", 0.3125),
            ("1/4A", 0.625),
            ("1/2A", 1.25),
            ("A", 2.5),
            ("B", 5.0),
            ("C", 10.0),
            ("D", 20.0),
            ("E", 40.0),
            ("F", 80.0),
            ("G", 160.0),
            ("H", 320.0),
            ("I", 640.0),
            ("J", 1280.0),
            ("K", 2560.0),
            ("L", 5120.0),
            ("M", 10240.0),
            ("N", 20480.0),
            ("O", 40960.0),
        ];
        for (label, limit) in table {
            let class: ImpulseClass = label.parse().unwrap();
            assert_eq!(class.upper_limit_ns(), limit, "{label}");
            assert_eq!(class.label(), label);
            assert_eq!(ImpulseClass::from_total_impulse(limit).unwrap(), class);
        }
    }

    #[test]
    fn labels_round_trip_through_serde() {
        for exponent in MIN_EXPONENT..=MAX_EXPONENT {
            let class = ImpulseClass { exponent };
            let json = serde_json::to_string(&class).unwrap();
            assert_eq!(serde_json::from_str::<ImpulseClass>(&json).unwrap(), class);
        }
        assert!(serde_json::from_str::<ImpulseClass>("\"1/16A\"").is_err());
        assert!("AA".parse::<ImpulseClass>().is_err());
    }

    #[test]
    fn rejects_impulses_outside_the_classes() {
        for bad in [
            0.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            1.25 * 2f64.powi(26) * 1.000_001,
        ] {
            assert!(ImpulseClass::from_total_impulse(bad).is_err(), "{bad}");
        }
        assert_eq!(
            ImpulseClass::from_total_impulse(f64::MIN_POSITIVE)
                .unwrap()
                .label(),
            "1/8A"
        );
    }
}
