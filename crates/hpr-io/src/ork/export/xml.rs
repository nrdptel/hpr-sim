//! Building the elements of a `.ork` document, and writing numbers so they read back exactly.

use super::super::document::{Element, Node};
use super::super::value::Dimension;

/// An empty element called `name`.
pub(crate) fn element(name: &str) -> Element {
    Element {
        name: name.to_owned(),
        attributes: Vec::new(),
        children: Vec::new(),
    }
}

/// An element called `name` holding `text`.
pub(crate) fn tag(name: &str, text: impl Into<String>) -> Element {
    let text = text.into();
    let mut element = element(name);
    if !text.is_empty() {
        element.children.push(Node::Text { text });
    }
    element
}

/// A number as text that reads back as the same `f64`.
///
/// Rust writes the shortest decimal that parses back to exactly the same `f64`, and never in
/// exponent form, so Java's `Double.parseDouble` reads it as the same number too. This is Loft
/// lesson L68's third part: Loft rounded every number to six decimals.
pub(crate) fn number(value: f64) -> String {
    format!("{value}")
}

/// A dimension as a `.ork` writes it: a number, `auto`, or `auto` with the number OpenRocket last
/// worked out.
pub(crate) fn dimension(value: Dimension) -> String {
    match value {
        Dimension::Stated { value } => number(value),
        Dimension::Automatic { cached: None } => "auto".to_owned(),
        Dimension::Automatic {
            cached: Some(cached),
        } => format!("auto {}", number(cached)),
    }
}

/// A flag as a `.ork` writes it.
pub(crate) fn flag(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

/// Adding children and attributes to an element as it is built.
pub(crate) trait Build {
    /// Adds `child` last.
    fn push(&mut self, child: Element) -> &mut Self;
    /// Adds `<name>text</name>` last.
    fn leaf(&mut self, name: &str, text: impl Into<String>) -> &mut Self;
    /// Adds `<name>value</name>` last, the number written exactly.
    fn number(&mut self, name: &str, value: f64) -> &mut Self;
    /// Adds `<name>…</name>` last when `value` is set.
    fn maybe_number(&mut self, name: &str, value: Option<f64>) -> &mut Self;
    /// Adds `<name>true</name>` or `<name>false</name>` last.
    fn flag(&mut self, name: &str, value: bool) -> &mut Self;
    /// Adds the attribute `name="value"` last.
    fn with_attribute(&mut self, name: &str, value: impl Into<String>) -> &mut Self;
}

impl Build for Element {
    fn push(&mut self, child: Element) -> &mut Self {
        self.children.push(Node::Element(child));
        self
    }

    fn leaf(&mut self, name: &str, text: impl Into<String>) -> &mut Self {
        self.push(tag(name, text))
    }

    fn number(&mut self, name: &str, value: f64) -> &mut Self {
        self.push(tag(name, number(value)))
    }

    fn maybe_number(&mut self, name: &str, value: Option<f64>) -> &mut Self {
        match value {
            Some(value) => self.number(name, value),
            None => self,
        }
    }

    fn flag(&mut self, name: &str, value: bool) -> &mut Self {
        self.push(tag(name, flag(value)))
    }

    fn with_attribute(&mut self, name: &str, value: impl Into<String>) -> &mut Self {
        self.attributes.push((name.to_owned(), value.into()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every number is written so it parses back to the same bits, however many digits that
    /// takes, and never in exponent form, which some readers of a `.ork` do not expect.
    #[test]
    fn numbers_read_back_to_the_same_bits() {
        for value in [
            0.1,
            0.1 + 0.2,
            1.0 / 3.0,
            0.0254 * 7.0 / 16.0,
            1e-9,
            -0.0,
            123_456.789_012_345_67,
            f64::MIN_POSITIVE,
            5e-324,
            1.797_693_134_862_315_7e308,
        ] {
            let text = number(value);
            assert!(!text.contains(['e', 'E']), "{text}");
            let back: f64 = text.parse().expect("a number");
            assert_eq!(back.to_bits(), value.to_bits(), "{value:e} wrote {text}");
        }
        assert_eq!(number(0.3), "0.3");
        assert_eq!(number(2.0), "2");
        assert_eq!(
            dimension(Dimension::Automatic {
                cached: Some(0.0125)
            }),
            "auto 0.0125"
        );
        assert_eq!(dimension(Dimension::Automatic { cached: None }), "auto");
    }
}
