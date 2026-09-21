//! Reading the values a `.ork` writes inside its elements.
//!
//! Every later step walks the document tree and asks it for numbers, counts, flags and
//! dimensions. Three things make that less simple than it sounds, and all three are mistakes Loft
//! made:
//!
//! - **A dimension may be automatic.** `<outerradius>auto 0.0125</outerradius>` means "OpenRocket
//!   works this out from the neighbours, and 0.0125 m is what it last worked out". Keeping only
//!   the number turns an automatic dimension into a hand-typed one the next time the design is
//!   saved ([Loft lesson L58][lessons]). [`Dimension`] keeps both.
//! - **A tag may have two names.** OpenRocket renamed several, and writes both for older readers.
//!   The reader takes the first name it finds, newest first ([Loft lesson L62][lessons]).
//! - **A stated zero is a value.** `<overridecd>0.0</overridecd>` means no drag at all, not "no
//!   override" — Loft read it as missing and charged the part full drag
//!   ([Loft lesson L63][lessons]).
//!
//! [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md

use serde::{Deserialize, Serialize};

use super::document::Element;
use super::warning::{Warning, WarningKind};

/// The two names OpenRocket writes a component's distance along its parent under.
///
/// `position` carries a `type` attribute and `axialoffset` a `method`, with the same vocabulary:
/// `top`, `middle`, `bottom`, `after`, `absolute`.
pub const AXIAL_OFFSET: [&str; 2] = ["axialoffset", "position"];

/// The two names for how many of an instanced component there are (fins, rail buttons, pods).
pub const INSTANCE_COUNT: [&str; 2] = ["instancecount", "fincount"];

/// The two names for the angle an instanced component sits at about the body axis.
pub const ANGLE_OFFSET: [&str; 2] = ["angleoffset", "radialdirection"];

/// The two names for the distance an instanced component sits off the body axis.
///
/// Unlike the other three, no file in the reference corpus writes both, so they have never been
/// seen to agree; see the [`.ork` page] for what is and is not measured.
///
/// [`.ork` page]: https://nrdptel.github.io/hpr-sim/format/ork.html
pub const RADIUS_OFFSET: [&str; 2] = ["radiusoffset", "radialposition"];

/// A number a `.ork` writes, which OpenRocket may be working out for itself.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Dimension {
    /// A number the designer typed.
    Stated {
        /// The number, in whatever unit the tag is written in.
        value: f64,
    },
    /// A number OpenRocket works out from the neighbouring components.
    Automatic {
        /// What it last worked out, when the file says (`auto 0.0125`). A bare `auto` gives
        /// `None`. Either way this is a cached answer, not an input: a reader that resolves the
        /// dimension itself should prefer its own.
        cached: Option<f64>,
    },
}

impl Dimension {
    /// The number, whether it was typed or cached. `None` for a bare `auto`.
    pub fn value(self) -> Option<f64> {
        match self {
            Self::Stated { value } => Some(value),
            Self::Automatic { cached } => cached,
        }
    }

    /// Whether OpenRocket works this dimension out for itself.
    pub fn is_automatic(self) -> bool {
        matches!(self, Self::Automatic { .. })
    }

    /// Reads a dimension from an element's text.
    fn read(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("auto") {
            let rest = rest.trim();
            if rest.is_empty() {
                return Some(Self::Automatic { cached: None });
            }
            return finite(rest).map(|cached| Self::Automatic {
                cached: Some(cached),
            });
        }
        finite(text).map(|value| Self::Stated { value })
    }
}

/// Reads a `.ork` element's child values, collecting a warning for anything it cannot.
///
/// Borrowed rather than owned so that walking a tree costs nothing: make one per element.
#[derive(Debug)]
pub struct Values<'a> {
    element: &'a Element,
    at: &'a str,
    warnings: &'a mut Vec<Warning>,
}

impl<'a> Values<'a> {
    /// Reads the children of `element`, reporting anything odd as happening at `at`.
    pub fn new(element: &'a Element, at: &'a str, warnings: &'a mut Vec<Warning>) -> Self {
        Self {
            element,
            at,
            warnings,
        }
    }

    /// The first child called one of `names`, newest name first.
    ///
    /// When more than one is present they are checked against each other: in the reference corpus
    /// OpenRocket writes both names on 777 elements and the text is the same on every one, so a
    /// disagreement is worth a warning. The first name still wins.
    pub fn element(&mut self, names: &[&str]) -> Option<&'a Element> {
        let mut found: Option<&'a Element> = None;
        for name in names {
            let Some(child) = self.element.child(name) else {
                continue;
            };
            match found {
                None => found = Some(child),
                Some(first) if first.text().trim() != child.text().trim() => {
                    let (winner, loser) = (first.name.clone(), child.name.clone());
                    self.warn(
                        WarningKind::Dropped,
                        format!(
                            "`{winner}` says `{}` and `{loser}` says `{}`; they are two names for \
                             one value, so `{winner}` was taken",
                            first.text().trim(),
                            child.text().trim()
                        ),
                    );
                }
                Some(_) => {}
            }
        }
        found
    }

    /// The text of the first child called one of `names`, trimmed. An empty tag gives `""`.
    pub fn word(&mut self, names: &[&str]) -> Option<String> {
        self.element(names)
            .map(|child| child.text().trim().to_owned())
    }

    /// A number. A tag whose text is not a finite number is ignored, with a warning.
    pub fn number(&mut self, names: &[&str]) -> Option<f64> {
        let child = self.element(names)?;
        let text = child.text();
        match finite(text.trim()) {
            Some(value) => Some(value),
            None => {
                let name = child.name.clone();
                self.warn(
                    WarningKind::Dropped,
                    format!(
                        "`{name}` says `{}`, which is not a number; it was ignored",
                        text.trim()
                    ),
                );
                None
            }
        }
    }

    /// A count. A negative or fractional one is ignored, with a warning.
    pub fn count(&mut self, names: &[&str]) -> Option<u32> {
        let value = self.number(names)?;
        let rounded = value.round();
        if rounded < 0.0 || rounded > f64::from(u32::MAX) || (value - rounded).abs() > 1e-9 {
            self.warn(
                WarningKind::Dropped,
                format!("a count of `{value}` is not a whole number of things; it was ignored"),
            );
            return None;
        }
        // The bounds above are what make this cast exact.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked to be a whole number within u32 on the line above"
        )]
        Some(rounded as u32)
    }

    /// A `true`/`false` flag. Anything else is ignored, with a warning.
    pub fn flag(&mut self, names: &[&str]) -> Option<bool> {
        let child = self.element(names)?;
        match child.text().trim() {
            "true" => Some(true),
            "false" => Some(false),
            other => {
                let name = child.name.clone();
                let other = other.to_owned();
                self.warn(
                    WarningKind::Dropped,
                    format!(
                        "`{name}` says `{other}`, which is neither true nor false; it was ignored"
                    ),
                );
                None
            }
        }
    }

    /// A dimension, which may be `auto` with or without the number OpenRocket last worked out.
    pub fn dimension(&mut self, names: &[&str]) -> Option<Dimension> {
        let child = self.element(names)?;
        let text = child.text();
        match Dimension::read(&text) {
            Some(dimension) => Some(dimension),
            None => {
                let name = child.name.clone();
                self.warn(
                    WarningKind::Dropped,
                    format!(
                        "`{name}` says `{}`, which is neither a number nor `auto`; it was ignored",
                        text.trim()
                    ),
                );
                None
            }
        }
    }

    /// The mass, centre of gravity and drag a component's own figures are replaced by.
    pub fn overrides(&mut self) -> Overrides {
        let mass_kg = self.number(&["overridemass"]);
        let cg_m = self.number(&["overridecg"]);
        let cd = self.number(&["overridecd"]);
        // The three flags were one until schema 1.9. No element in the reference corpus carries
        // both forms, so the older one is read as setting all three, which is what it meant.
        let all = self.flag(&["overridesubcomponents"]);
        if all.is_some() {
            self.warn(
                WarningKind::Unusual,
                "this component uses the single `overridesubcomponents` flag that OpenRocket \
                 replaced with one flag per quantity; it was read as setting all three",
            );
        }
        Overrides {
            mass_kg,
            cg_m,
            cd,
            subcomponents_mass: self.flag(&["overridesubcomponentsmass"]).or(all),
            subcomponents_cg: self.flag(&["overridesubcomponentscg"]).or(all),
            subcomponents_cd: self.flag(&["overridesubcomponentscd"]).or(all),
        }
    }

    fn warn(&mut self, kind: WarningKind, message: impl Into<String>) {
        self.warnings.push(Warning::new(self.at, kind, message));
    }
}

/// What a component says its own mass, centre of gravity and drag coefficient are, instead of
/// what its geometry and material would give.
///
/// The three values and the three flags are independent, which is [Loft lesson L63: Loft read
/// neither `overridecd` nor `overridesubcomponentscg`, so a part set to a drag coefficient of zero
/// was still charged full drag][lessons]. A stated `0.0` here is an override to zero, not a
/// missing one.
///
/// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Overrides {
    /// The mass the component is declared to have, in kilograms.
    pub mass_kg: Option<f64>,
    /// Where its centre of gravity is declared to be, in metres from its fore end.
    pub cg_m: Option<f64>,
    /// The drag coefficient it is declared to have.
    pub cd: Option<f64>,
    /// Whether the mass override covers the components inside this one too.
    pub subcomponents_mass: Option<bool>,
    /// Whether the centre-of-gravity override covers them.
    pub subcomponents_cg: Option<bool>,
    /// Whether the drag override covers them.
    pub subcomponents_cd: Option<bool>,
}

/// Rust also parses `inf` and `NaN`, which no design file means.
fn finite(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}
