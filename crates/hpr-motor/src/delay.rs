//! Ejection delays: the time from burnout to the ejection charge, or none for a plugged motor.
//!
//! Motor files and catalogs write a motor's available delays as one string. The RASP spec gives
//! "delays separated by dashes", with `0` for an ejection charge with no delay and `P` for plugged
//! (`docs/format/eng.md`). Real files also use commas, a trailing `P`, and `100` or `1000` as
//! plugged markers; checked against ThrustCurve.org's metadata, `100` and `1000` nearly always mean
//! plugged, and so does `0` more often than not (counts in `docs/format/eng.md` and `rse.md`). So
//! this reader:
//!
//! - splits on `-` and `,`, dropping empty pieces with a warning;
//! - reads `P` (either case), `100` and `1000` as [`Delay::Plugged`];
//! - reads `0` as [`Delay::ZeroOrPlugged`], with a warning, so that it can't become an ejection at
//!   burnout without a decision;
//! - reads other numbers as [`Delay::Seconds`].
//!
//! The raw string stays in the file model so writers reproduce it exactly; physics should prefer
//! the catalog's delays.

use serde::{Deserialize, Serialize};

use crate::text::WarningKind;

/// One available delay setting. Serialized with a `kind` tag and the seconds as `value`:
/// `{"kind":"seconds","value":6.0}`, `{"kind":"plugged"}`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Delay {
    /// The ejection charge fires this many seconds after burnout (positive).
    Seconds(f64),
    /// No ejection charge: the forward closure is plugged.
    Plugged,
    /// A `0`: the RASP spec means an ejection charge at burnout, but most files mean plugged
    /// (`docs/format/eng.md`). The user or the catalog has to settle which.
    ZeroOrPlugged,
}

/// A delay string read into settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelayList {
    /// The settings in the order written (some files list them longest first).
    pub delays: Vec<Delay>,
    /// Problems found while reading: an empty or unreadable piece ([`WarningKind::Dropped`], and
    /// left out of `delays`), or an ambiguous `0` ([`WarningKind::Unusual`], kept as
    /// [`Delay::ZeroOrPlugged`]).
    pub warnings: Vec<DelayWarning>,
}

/// A problem found while reading a delay string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelayWarning {
    /// Whether a piece was dropped.
    pub kind: WarningKind,
    /// What was found.
    pub message: String,
}

impl DelayList {
    /// Reads a delay string such as `6-10-14`, `5,8,11`, `P`, `6-10-14-P` or `1000`. Any number
    /// equal to 100 or 1000 (`1000.` too) is a plugged marker.
    pub fn parse(raw: &str) -> Self {
        let mut delays = Vec::new();
        let mut warnings = Vec::new();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Self { delays, warnings };
        }
        for piece in trimmed.split(['-', ',']) {
            let piece = piece.trim();
            match piece {
                "" => warnings.push(DelayWarning {
                    kind: WarningKind::Dropped,
                    message: format!("empty delay in {raw:?} ignored"),
                }),
                "P" | "p" => delays.push(Delay::Plugged),
                _ => match piece.parse::<f64>() {
                    Ok(marker) if marker == 100.0 || marker == 1000.0 => {
                        delays.push(Delay::Plugged)
                    }
                    Ok(0.0) => {
                        warnings.push(DelayWarning {
                            kind: WarningKind::Unusual,
                            message: format!(
                                "delay 0 in {raw:?} is ambiguous: the RASP spec means an ejection \
                                 charge with no delay, but files mostly mean plugged"
                            ),
                        });
                        delays.push(Delay::ZeroOrPlugged);
                    }
                    Ok(seconds) if seconds.is_finite() && seconds > 0.0 => {
                        delays.push(Delay::Seconds(seconds));
                    }
                    _ => warnings.push(DelayWarning {
                        kind: WarningKind::Dropped,
                        message: format!("unreadable delay {piece:?} in {raw:?} ignored"),
                    }),
                },
            }
        }
        Self { delays, warnings }
    }

    /// Whether any setting is plugged ([`Delay::Plugged`]; an ambiguous `0` doesn't count).
    pub fn has_plugged(&self) -> bool {
        self.delays.contains(&Delay::Plugged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_lists_and_markers() {
        let list = DelayList::parse("6-10-14");
        assert_eq!(
            list.delays,
            [
                Delay::Seconds(6.0),
                Delay::Seconds(10.0),
                Delay::Seconds(14.0)
            ]
        );
        assert!(list.warnings.is_empty());
        assert_eq!(DelayList::parse("P").delays, [Delay::Plugged]);
        assert_eq!(DelayList::parse("1000.").delays, [Delay::Plugged]);
        assert_eq!(DelayList::parse("100.0").delays, [Delay::Plugged]);
        assert_eq!(DelayList::parse(" p ").delays, [Delay::Plugged]);
        assert_eq!(
            DelayList::parse("10,14,1000").delays,
            [Delay::Seconds(10.0), Delay::Seconds(14.0), Delay::Plugged]
        );
        assert!(DelayList::parse("").delays.is_empty());
        assert!(!DelayList::parse("6").has_plugged());
    }

    #[test]
    fn serde_form_is_tagged() {
        let delays = [Delay::Seconds(6.0), Delay::Plugged, Delay::ZeroOrPlugged];
        let json = serde_json::to_string(&delays).unwrap();
        assert_eq!(
            json,
            r#"[{"kind":"seconds","value":6.0},{"kind":"plugged"},{"kind":"zero_or_plugged"}]"#
        );
        assert_eq!(serde_json::from_str::<Vec<Delay>>(&json).unwrap(), delays);
    }

    #[test]
    fn malformed_pieces_are_dropped_with_warnings() {
        let list = DelayList::parse("4-7-10,");
        assert_eq!(list.delays.len(), 3);
        assert_eq!(list.warnings.len(), 1);
        let list = DelayList::parse("1-3--4");
        assert_eq!(list.delays.len(), 3);
        assert_eq!(list.warnings.len(), 1);
        let list = DelayList::parse("-");
        assert!(list.delays.is_empty());
        assert_eq!(list.warnings.len(), 2);
        let list = DelayList::parse("S-M-L");
        assert!(list.delays.is_empty());
        assert_eq!(list.warnings.len(), 3);
        let list = DelayList::parse("0");
        assert_eq!(list.delays, [Delay::ZeroOrPlugged]);
        assert!(!list.has_plugged());
        assert_eq!(list.warnings.len(), 1);
        assert_eq!(list.warnings[0].kind, WarningKind::Unusual);
        assert_eq!(DelayList::parse("0.0-6").delays[0], Delay::ZeroOrPlugged);
        assert_eq!(DelayList::parse("x").warnings[0].kind, WarningKind::Dropped);
    }
}
