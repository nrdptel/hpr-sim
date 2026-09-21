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
/// `position` carries a `type` attribute and `axialoffset` a `method`, with the same vocabulary —
/// `top`, `middle`, `bottom`, `after`, `absolute` — and **Observed:** on all 642 elements of the
/// reference corpus that carry both, the two agree on the text *and* on that attribute.
pub const AXIAL_OFFSET: [&str; 2] = ["axialoffset", "position"];

/// The two names for how many of an instanced component there are (fins, rail buttons, pods).
///
/// **Observed:** 109 elements carry both, agreeing every time. Neither carries an attribute, so a
/// count is the one value here that a rename cannot change the meaning of.
pub const INSTANCE_COUNT: [&str; 2] = ["instancecount", "fincount"];

// `angleoffset`/`radialdirection` and `radiusoffset`/`radialposition` are deliberately **not**
// here. They look like the two pairs above, and they are not: the newer name of each carries a
// `method` attribute — the frame the number is measured in — that the older name never carries.
// `cargo xtask ork` measures it: of the 26 elements with both `angleoffset` and `radialdirection`
// the texts agree every time and the frames differ every time, and `radiusoffset` (106 elements)
// and `radialposition` (542) are never written together at all. Reading one as the other would
// silently move a component, so settling what the older name's frame is belongs to the milestone
// that places components (M3.1b2), with a source, not to a constant here. See the `.ork` page:
// <https://nrdptel.github.io/hpr-sim/format/ork.html>.

/// A number a `.ork` writes, which OpenRocket may be working out for itself.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
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

    /// Reads a dimension from an element's text, or `None` when the text is neither a finite
    /// number nor `auto`.
    ///
    /// Only a tag that *holds* a dimension should be read this way. `<ignitionevent>automatic
    /// </ignitionevent>` is a word, not a number, and gives `None` rather than an automatic
    /// dimension.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("auto") {
            if rest.is_empty() {
                return Some(Self::Automatic { cached: None });
            }
            // `auto` is a word: `auto 0.025` is a cached dimension, `auto-1` and `automatic` are
            // not this tag's business.
            let cached = rest.strip_prefix(char::is_whitespace)?.trim();
            return finite(cached).map(|cached| Self::Automatic {
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
    /// When more than one is present they are checked against each other, on the text **and** on
    /// the `method`/`type` attribute that says what the text is measured from: in the reference
    /// corpus OpenRocket agrees with itself on both, every time. A disagreement is therefore worth
    /// a warning, and the first name still wins.
    pub fn element(&mut self, names: &[&str]) -> Option<&'a Element> {
        let mut found: Option<&'a Element> = None;
        for name in names {
            super::reads::note(self.element, name);
            let Some(child) = self.element.child(name) else {
                continue;
            };
            let Some(first) = found else {
                found = Some(child);
                continue;
            };
            let (winner, loser) = (first.name.clone(), child.name.clone());
            if first.text().trim() != child.text().trim() {
                self.warn(
                    WarningKind::Dropped,
                    format!(
                        "`{winner}` says `{}` and `{loser}` says `{}`; they are two names for one \
                         value, so `{winner}` was taken",
                        first.text().trim(),
                        child.text().trim()
                    ),
                );
            } else if frame(first) != frame(child) {
                self.warn(
                    WarningKind::Dropped,
                    format!(
                        "`{winner}` and `{loser}` both say `{}` but measure it from {} and {}; \
                         `{winner}` was taken",
                        first.text().trim(),
                        named(frame(first)),
                        named(frame(child))
                    ),
                );
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
        match Dimension::parse(&text) {
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

    /// Records a warning about the element being read, for a caller that decided something this
    /// reader could not.
    pub fn warn_at(&mut self, kind: WarningKind, message: impl Into<String>) {
        self.warn(kind, message.into());
    }

    /// The mass, centre of gravity and drag a component's own figures are replaced by.
    pub fn overrides(&mut self) -> Overrides {
        let mass_kg = self.number(&["overridemass"]);
        let cg_m = self.number(&["overridecg"]);
        let cd = self.number(&["overridecd"]);
        // The three flags were one until schema 1.9. No element in the reference corpus carries
        // both forms, so the older one is read as setting all three, which is what it meant.
        let all = self.flag(&["overridesubcomponents"]);
        let per_quantity = [
            ("mass", self.flag(&["overridesubcomponentsmass"])),
            ("centre of gravity", self.flag(&["overridesubcomponentscg"])),
            ("drag", self.flag(&["overridesubcomponentscd"])),
        ];
        if all.is_some() {
            let taken: Vec<&str> = per_quantity
                .iter()
                .filter(|(_, own)| own.is_none())
                .map(|(what, _)| *what)
                .collect();
            self.warn(
                WarningKind::Unusual,
                format!(
                    "this component uses the single `overridesubcomponents` flag that OpenRocket \
                     replaced with one flag per quantity; it was read as setting {}",
                    match taken.as_slice() {
                        [] => "nothing, since each quantity has a flag of its own".to_owned(),
                        [one] => format!("the {one} flag"),
                        many => format!("the {} flags", many.join(", the ")),
                    }
                ),
            );
        }
        let [(_, mass), (_, cg), (_, drag)] = per_quantity;
        Overrides {
            mass_kg,
            cg_m,
            cd,
            subcomponents_mass: mass.or(all),
            subcomponents_cg: cg.or(all),
            subcomponents_cd: drag.or(all),
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
#[non_exhaustive]
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

/// What a placement tag says its number is measured from: `method` on the newer name of a pair,
/// `type` on the older.
fn frame(element: &Element) -> Option<&str> {
    element
        .attribute("method")
        .or_else(|| element.attribute("type"))
}

fn named(frame: Option<&str>) -> String {
    frame.map_or_else(|| "nowhere stated".to_owned(), |frame| format!("`{frame}`"))
}

/// Rust also parses `inf` and `NaN`, which no design file means.
fn finite(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}
