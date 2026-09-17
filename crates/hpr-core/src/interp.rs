//! One-dimensional lookup tables with an explicit extrapolation policy.
//!
//! A [`Table1D`] holds knots `(x_i, y_i)` with strictly increasing `x_i` and interpolates between
//! them either piecewise linearly or with a natural cubic spline. Outside `[x_0, x_{n-1}]` it
//! follows its [`Extrapolation`] policy, and every [`Table1D::lookup`] reports whether it
//! extrapolated, so callers can warn instead of silently trusting a clamped value.
//!
//! **Natural cubic spline.** Each interval is a cubic Hermite polynomial in the knot values `y_i`
//! and knot slopes `s_i`. The slopes solve the tridiagonal system that makes the second derivative
//! continuous at the interior knots (the slope form of cubic spline interpolation; see C. de Boor,
//! *A Practical Guide to Splines*, rev. ed., Springer, 2001, ch. IV):
//!
//! ```text
//! h_i s_{i-1} + 2 (h_{i-1} + h_i) s_i + h_{i-1} s_{i+1} = 3 (h_i δ_{i-1} + h_{i-1} δ_i),  0 < i < n-1
//! ```
//!
//! with `h_i = x_{i+1} - x_i` and `δ_i = (y_{i+1} - y_i) / h_i`, closed by the natural ("free
//! end") conditions `y''(x_0) = y''(x_{n-1}) = 0`:
//!
//! ```text
//! 2 s_0 + s_1 = 3 δ_0,        s_{n-2} + 2 s_{n-1} = 3 δ_{n-2}
//! ```
//!
//! The system is strictly diagonally dominant, so the Thomas algorithm solves it stably without
//! pivoting. `docs/physics/interpolation.md` has the derivation and the tests that pin it.

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// How a table fills in between its knots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Interpolation {
    /// Piecewise linear: continuous, with slope jumps at the knots.
    #[default]
    Linear,
    /// Natural cubic spline: twice continuously differentiable, with zero second derivative at
    /// both end knots. It can overshoot between knots when the data turn sharply.
    NaturalCubic,
}

/// What a lookup outside `[x_0, x_{n-1}]` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Extrapolation {
    /// Hold the end value: `y(x) = y_0` below the table and `y_{n-1}` above it.
    #[default]
    Clamp,
    /// Continue along the slope at the end knot: the end segment's slope for a linear table,
    /// the spline's end slope `s_0` or `s_{n-1}` for a cubic one.
    Linear,
    /// Refuse the lookup with [`CoreError::OutOfRange`].
    Error,
}

/// Which end of a table a lookup fell beyond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Below the first abscissa.
    Below,
    /// Above the last abscissa.
    Above,
}

/// The result of a table lookup.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Lookup {
    /// The interpolated or extrapolated value.
    pub value: f64,
    /// `Some` when the abscissa was outside the table and the extrapolation policy produced the
    /// value; `None` inside the table, ends included.
    pub extrapolated: Option<Side>,
}

/// A one-dimensional lookup table `y(x)`.
///
/// Built through [`Table1D::new`], which checks the knots; it serializes as its knots and
/// policies (`x`, `y`, `interpolation`, `extrapolation`) and re-checks them when deserialized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TableData", into = "TableData")]
pub struct Table1D {
    xs: Vec<f64>,
    ys: Vec<f64>,
    interpolation: Interpolation,
    extrapolation: Extrapolation,
    /// `dy/dx` at each knot for the cubic forms; empty for a linear table.
    slopes: Vec<f64>,
}

/// The serialized form of a [`Table1D`].
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TableData {
    x: Vec<f64>,
    y: Vec<f64>,
    #[serde(default)]
    interpolation: Interpolation,
    #[serde(default)]
    extrapolation: Extrapolation,
}

impl TryFrom<TableData> for Table1D {
    type Error = CoreError;

    fn try_from(data: TableData) -> Result<Self, CoreError> {
        Table1D::new(data.x, data.y, data.interpolation, data.extrapolation)
    }
}

impl From<Table1D> for TableData {
    fn from(table: Table1D) -> Self {
        TableData {
            x: table.xs,
            y: table.ys,
            interpolation: table.interpolation,
            extrapolation: table.extrapolation,
        }
    }
}

impl Table1D {
    /// The fewest knots a table accepts.
    pub const MIN_KNOTS: usize = 2;

    /// Builds a table from its abscissae `xs` and ordinates `ys`.
    ///
    /// # Errors
    ///
    /// - [`CoreError::TableLengthMismatch`] if the columns differ in length.
    /// - [`CoreError::TableTooShort`] with fewer than [`Table1D::MIN_KNOTS`] knots.
    /// - [`CoreError::TableNotFinite`] if any value is NaN or infinite, or a secant slope
    ///   `(y_{i+1} - y_i) / (x_{i+1} - x_i)` overflows.
    /// - [`CoreError::TableNotIncreasing`] unless `xs` strictly increases.
    pub fn new(
        xs: Vec<f64>,
        ys: Vec<f64>,
        interpolation: Interpolation,
        extrapolation: Extrapolation,
    ) -> Result<Self, CoreError> {
        if xs.len() != ys.len() {
            return Err(CoreError::TableLengthMismatch {
                xs: xs.len(),
                ys: ys.len(),
            });
        }
        if xs.len() < Self::MIN_KNOTS {
            return Err(CoreError::TableTooShort {
                min: Self::MIN_KNOTS,
                got: xs.len(),
            });
        }
        if let Some(index) = xs
            .iter()
            .zip(&ys)
            .position(|(x, y)| !x.is_finite() || !y.is_finite())
        {
            return Err(CoreError::TableNotFinite { index });
        }
        if let Some(index) = xs.windows(2).position(|w| w[1] <= w[0]) {
            return Err(CoreError::TableNotIncreasing { index: index + 1 });
        }
        let secants: Vec<f64> = xs
            .windows(2)
            .zip(ys.windows(2))
            .map(|(x, y)| (y[1] - y[0]) / (x[1] - x[0]))
            .collect();
        if let Some(index) = secants.iter().position(|s| !s.is_finite()) {
            return Err(CoreError::TableNotFinite { index: index + 1 });
        }
        let slopes = match interpolation {
            Interpolation::Linear => Vec::new(),
            Interpolation::NaturalCubic => natural_spline_slopes(&xs, &secants),
        };
        Ok(Self {
            xs,
            ys,
            interpolation,
            extrapolation,
            slopes,
        })
    }

    /// The abscissae, strictly increasing.
    pub fn xs(&self) -> &[f64] {
        &self.xs
    }

    /// The ordinates.
    pub fn ys(&self) -> &[f64] {
        &self.ys
    }

    /// The interpolation between knots.
    pub fn interpolation(&self) -> Interpolation {
        self.interpolation
    }

    /// The extrapolation policy.
    pub fn extrapolation(&self) -> Extrapolation {
        self.extrapolation
    }

    /// The first and last abscissae.
    pub fn domain(&self) -> (f64, f64) {
        (self.first_x(), self.last_x())
    }

    /// Looks up `y(x)`, reporting whether the value was extrapolated.
    ///
    /// # Errors
    ///
    /// - [`CoreError::NanLookup`] if `x` is NaN.
    /// - [`CoreError::OutOfRange`] if `x` is outside the table and the policy is
    ///   [`Extrapolation::Error`].
    pub fn lookup(&self, x: f64) -> Result<Lookup, CoreError> {
        if x.is_nan() {
            return Err(CoreError::NanLookup);
        }
        let (first, last) = self.domain();
        let side = if x < first {
            Some(Side::Below)
        } else if x > last {
            Some(Side::Above)
        } else {
            None
        };
        let value = match side {
            None => self.interpolate(x),
            Some(side) => {
                let (x_end, y_end, slope) = match side {
                    Side::Below => (first, self.first_y(), self.end_slope(Side::Below)),
                    Side::Above => (last, self.last_y(), self.end_slope(Side::Above)),
                };
                match self.extrapolation {
                    Extrapolation::Clamp => y_end,
                    Extrapolation::Linear => y_end + slope * (x - x_end),
                    Extrapolation::Error => {
                        return Err(CoreError::OutOfRange {
                            x,
                            min: first,
                            max: last,
                        });
                    }
                }
            }
        };
        Ok(Lookup {
            value,
            extrapolated: side,
        })
    }

    /// Looks up `y(x)`; the same as [`Table1D::lookup`] without the extrapolation flag.
    ///
    /// # Errors
    ///
    /// As [`Table1D::lookup`].
    pub fn eval(&self, x: f64) -> Result<f64, CoreError> {
        self.lookup(x).map(|lookup| lookup.value)
    }

    fn first_x(&self) -> f64 {
        self.xs.first().copied().unwrap_or(f64::NAN)
    }

    fn last_x(&self) -> f64 {
        self.xs.last().copied().unwrap_or(f64::NAN)
    }

    fn first_y(&self) -> f64 {
        self.ys.first().copied().unwrap_or(f64::NAN)
    }

    fn last_y(&self) -> f64 {
        self.ys.last().copied().unwrap_or(f64::NAN)
    }

    /// The slope used to extrapolate beyond one end.
    fn end_slope(&self, side: Side) -> f64 {
        let n = self.xs.len();
        match (self.interpolation, side) {
            (Interpolation::Linear, Side::Below) => secant(&self.xs, &self.ys, 0),
            (Interpolation::Linear, Side::Above) => secant(&self.xs, &self.ys, n - 2),
            (Interpolation::NaturalCubic, Side::Below) => self.slopes[0],
            (Interpolation::NaturalCubic, Side::Above) => self.slopes[n - 1],
        }
    }

    /// Interpolates at `x` inside `[x_0, x_{n-1}]`.
    fn interpolate(&self, x: f64) -> f64 {
        // The interval `[x_i, x_{i+1}]` holding x; the last interval also holds `x_{n-1}`.
        // `new` guarantees at least two knots, so `n - 2` doesn't underflow.
        let n = self.xs.len();
        let i = self.xs.partition_point(|&xi| xi <= x).clamp(1, n - 1) - 1;
        let (x0, x1) = (self.xs[i], self.xs[i + 1]);
        let (y0, y1) = (self.ys[i], self.ys[i + 1]);
        let h = x1 - x0;
        let t = (x - x0) / h;
        match self.interpolation {
            // This form returns y0 at t = 0 and y1 at t = 1 exactly.
            Interpolation::Linear => (1.0 - t) * y0 + t * y1,
            Interpolation::NaturalCubic => {
                // Cubic Hermite basis on [0, 1]; each basis function is exactly 0 or 1 at the
                // ends, so the knots are reproduced exactly.
                let u = 1.0 - t;
                let h00 = (1.0 + 2.0 * t) * u * u;
                let h10 = t * u * u;
                let h01 = t * t * (3.0 - 2.0 * t);
                let h11 = -t * t * u;
                h00 * y0 + h10 * h * self.slopes[i] + h01 * y1 + h11 * h * self.slopes[i + 1]
            }
        }
    }
}

/// The secant slope of interval `i`.
fn secant(xs: &[f64], ys: &[f64], i: usize) -> f64 {
    (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i])
}

/// Knot slopes of the natural cubic spline, by the Thomas algorithm on the system in the module
/// docs. `secants[i]` is `δ_i`; the caller guarantees `xs.len() >= 2` and finite secants.
fn natural_spline_slopes(xs: &[f64], secants: &[f64]) -> Vec<f64> {
    let n = xs.len();
    // Row i is `lower[i] s_{i-1} + diag[i] s_i + upper[i] s_{i+1} = rhs[i]`.
    let mut lower = vec![0.0; n];
    let mut diag = vec![0.0; n];
    let mut upper = vec![0.0; n];
    let mut rhs = vec![0.0; n];
    diag[0] = 2.0;
    upper[0] = 1.0;
    rhs[0] = 3.0 * secants[0];
    for i in 1..n - 1 {
        let h_prev = xs[i] - xs[i - 1];
        let h_next = xs[i + 1] - xs[i];
        lower[i] = h_next;
        diag[i] = 2.0 * (h_prev + h_next);
        upper[i] = h_prev;
        rhs[i] = 3.0 * (h_next * secants[i - 1] + h_prev * secants[i]);
    }
    lower[n - 1] = 1.0;
    diag[n - 1] = 2.0;
    rhs[n - 1] = 3.0 * secants[n - 2];

    // Forward sweep. Strict diagonal dominance keeps every pivot positive.
    for i in 1..n {
        let factor = lower[i] / diag[i - 1];
        diag[i] -= factor * upper[i - 1];
        rhs[i] -= factor * rhs[i - 1];
    }
    // Back substitution.
    let mut slopes = vec![0.0; n];
    slopes[n - 1] = rhs[n - 1] / diag[n - 1];
    for i in (0..n - 1).rev() {
        slopes[i] = (rhs[i] - upper[i] * slopes[i + 1]) / diag[i];
    }
    slopes
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn table(xs: &[f64], ys: &[f64], i: Interpolation, e: Extrapolation) -> Table1D {
        Table1D::new(xs.to_vec(), ys.to_vec(), i, e).unwrap()
    }

    /// Second derivative of the Hermite cubic on interval `i`, at its left (`t = 0`) or right
    /// (`t = 1`) end.
    fn second_derivative(table: &Table1D, i: usize, right: bool) -> f64 {
        let h = table.xs[i + 1] - table.xs[i];
        let d = secant(&table.xs, &table.ys, i);
        let (s0, s1) = (table.slopes[i], table.slopes[i + 1]);
        if right {
            (-6.0 * d + 2.0 * s0 + 4.0 * s1) / h
        } else {
            (6.0 * d - 4.0 * s0 - 2.0 * s1) / h
        }
    }

    #[test]
    fn rejects_malformed_knots() {
        let lin = Interpolation::Linear;
        let clamp = Extrapolation::Clamp;
        assert_eq!(
            Table1D::new(vec![0.0], vec![1.0], lin, clamp),
            Err(CoreError::TableTooShort { min: 2, got: 1 })
        );
        assert_eq!(
            Table1D::new(vec![0.0, 1.0], vec![1.0], lin, clamp),
            Err(CoreError::TableLengthMismatch { xs: 2, ys: 1 })
        );
        assert_eq!(
            Table1D::new(vec![0.0, 1.0, 1.0], vec![1.0, 2.0, 3.0], lin, clamp),
            Err(CoreError::TableNotIncreasing { index: 2 })
        );
        assert_eq!(
            Table1D::new(vec![0.0, f64::NAN], vec![1.0, 2.0], lin, clamp),
            Err(CoreError::TableNotFinite { index: 1 })
        );
        assert_eq!(
            Table1D::new(vec![0.0, 1.0], vec![f64::INFINITY, 2.0], lin, clamp),
            Err(CoreError::TableNotFinite { index: 0 })
        );
        // A secant that overflows: a huge rise over a subnormal run.
        assert_eq!(
            Table1D::new(vec![0.0, 1e-310], vec![-1e300, 1e300], lin, clamp),
            Err(CoreError::TableNotFinite { index: 1 })
        );
    }

    #[test]
    fn linear_interpolates_and_flags_extrapolation() {
        let t = table(
            &[0.0, 1.0, 3.0],
            &[0.0, 10.0, 0.0],
            Interpolation::Linear,
            Extrapolation::Linear,
        );
        assert_eq!(t.eval(0.5).unwrap(), 5.0);
        assert_eq!(t.eval(2.0).unwrap(), 5.0);
        assert_eq!(
            t.lookup(3.0).unwrap(),
            Lookup {
                value: 0.0,
                extrapolated: None
            }
        );
        assert_eq!(
            t.lookup(-1.0).unwrap(),
            Lookup {
                value: -10.0,
                extrapolated: Some(Side::Below)
            }
        );
        assert_eq!(
            t.lookup(4.0).unwrap(),
            Lookup {
                value: -5.0,
                extrapolated: Some(Side::Above)
            }
        );
        assert_eq!(t.lookup(f64::NAN), Err(CoreError::NanLookup));

        let clamped = table(
            &[0.0, 1.0],
            &[2.0, 4.0],
            Interpolation::Linear,
            Extrapolation::Clamp,
        );
        assert_eq!(clamped.eval(-5.0).unwrap(), 2.0);
        assert_eq!(clamped.eval(f64::INFINITY).unwrap(), 4.0);

        let strict = table(
            &[0.0, 1.0],
            &[2.0, 4.0],
            Interpolation::Linear,
            Extrapolation::Error,
        );
        assert_eq!(
            strict.lookup(1.5),
            Err(CoreError::OutOfRange {
                x: 1.5,
                min: 0.0,
                max: 1.0
            })
        );
        assert_eq!(strict.eval(1.0).unwrap(), 4.0);
    }

    /// Three equally spaced knots have a closed-form natural spline. With `h = 1`,
    /// `y = (0, 1, 0)`, the system gives `s_0 = 3/2`, `s_1 = 0`, `s_2 = -3/2`, so on `[0, 1]`
    /// the spline is `y(t) = 3t/2 - t³/2` (zero curvature at 0, `y(1) = 1`, `y'(1) = 0`).
    #[test]
    fn natural_spline_matches_the_three_knot_closed_form() {
        let t = table(
            &[0.0, 1.0, 2.0],
            &[0.0, 1.0, 0.0],
            Interpolation::NaturalCubic,
            Extrapolation::Linear,
        );
        assert_eq!(t.slopes, vec![1.5, 0.0, -1.5]);
        for k in 0..=10 {
            let x = f64::from(k) / 10.0;
            let exact = 1.5 * x - 0.5 * x.powi(3);
            assert!((t.eval(x).unwrap() - exact).abs() < 1e-15, "x = {x}");
            // Symmetric about x = 1.
            assert!(
                (t.eval(2.0 - x).unwrap() - exact).abs() < 1e-15,
                "x = {}",
                2.0 - x
            );
        }
        // Linear extrapolation continues along the end slopes.
        assert_eq!(t.eval(-2.0).unwrap(), -3.0);
        assert_eq!(t.eval(3.0).unwrap(), -1.5);
    }

    /// A strictly increasing grid of 2 to 24 knots with gaps between 1e-3 and 10.
    fn knots() -> impl Strategy<Value = (Vec<f64>, Vec<f64>)> {
        (2usize..24).prop_flat_map(|n| {
            (
                -1e3..1e3f64,
                prop::collection::vec(1e-3..10.0f64, n - 1),
                prop::collection::vec(-1e3..1e3f64, n),
            )
                .prop_map(|(start, gaps, ys)| {
                    let mut xs = vec![start];
                    for gap in gaps {
                        let last = xs[xs.len() - 1];
                        xs.push(last + gap);
                    }
                    (xs, ys)
                })
        })
    }

    proptest! {
        #[test]
        fn both_kinds_reproduce_the_knots((xs, ys) in knots()) {
            for kind in [Interpolation::Linear, Interpolation::NaturalCubic] {
                let t = table(&xs, &ys, kind, Extrapolation::Error);
                for (x, y) in xs.iter().zip(&ys) {
                    prop_assert_eq!(t.lookup(*x).unwrap(), Lookup { value: *y, extrapolated: None });
                }
            }
        }

        #[test]
        fn linear_stays_within_each_interval((xs, ys) in knots(), frac in 0.0..1.0f64) {
            let t = table(&xs, &ys, Interpolation::Linear, Extrapolation::Error);
            for i in 0..xs.len() - 1 {
                let x = xs[i] + frac * (xs[i + 1] - xs[i]);
                let y = t.eval(x).unwrap();
                let (lo, hi) = (ys[i].min(ys[i + 1]), ys[i].max(ys[i + 1]));
                prop_assert!(y >= lo - 1e-9 && y <= hi + 1e-9, "{y} outside [{lo}, {hi}]");
            }
        }

        #[test]
        fn natural_spline_is_c2_with_free_ends((xs, ys) in knots()) {
            let t = table(&xs, &ys, Interpolation::NaturalCubic, Extrapolation::Clamp);
            let n = xs.len();
            let scale = ys.iter().fold(1.0f64, |m, y| m.max(y.abs()))
                / xs.windows(2).fold(f64::INFINITY, |m, w| m.min(w[1] - w[0])).powi(2);
            prop_assert!(second_derivative(&t, 0, false).abs() <= 1e-9 * scale);
            prop_assert!(second_derivative(&t, n - 2, true).abs() <= 1e-9 * scale);
            for i in 1..n - 1 {
                let left = second_derivative(&t, i - 1, true);
                let right = second_derivative(&t, i, false);
                prop_assert!((left - right).abs() <= 1e-9 * scale, "knot {i}: {left} vs {right}");
            }
        }

        #[test]
        fn natural_spline_reproduces_straight_lines(
            (xs, _) in knots(), a in -100.0..100.0f64, b in -100.0..100.0f64, frac in 0.0..1.0f64,
        ) {
            let ys: Vec<f64> = xs.iter().map(|x| a + b * x).collect();
            let t = table(&xs, &ys, Interpolation::NaturalCubic, Extrapolation::Linear);
            let (first, last) = t.domain();
            for x in [first + frac * (last - first), first - 5.0, last + 5.0] {
                let exact = a + b * x;
                prop_assert!((t.eval(x).unwrap() - exact).abs() <= 1e-9 * (1.0 + exact.abs()));
            }
        }

        #[test]
        fn extrapolation_follows_the_policy((xs, ys) in knots(), beyond in 1e-6..100.0f64) {
            let n = xs.len();
            for kind in [Interpolation::Linear, Interpolation::NaturalCubic] {
                let below = xs[0] - beyond;
                let above = xs[n - 1] + beyond;

                let clamp = table(&xs, &ys, kind, Extrapolation::Clamp);
                prop_assert_eq!(clamp.lookup(below).unwrap(), Lookup { value: ys[0], extrapolated: Some(Side::Below) });
                prop_assert_eq!(clamp.lookup(above).unwrap(), Lookup { value: ys[n - 1], extrapolated: Some(Side::Above) });

                let strict = table(&xs, &ys, kind, Extrapolation::Error);
                prop_assert!(matches!(strict.lookup(below), Err(CoreError::OutOfRange { .. })), "below");
                prop_assert!(matches!(strict.lookup(above), Err(CoreError::OutOfRange { .. })), "above");

                // Linear extrapolation is continuous with the end knot and moves along the end slope.
                let line = table(&xs, &ys, kind, Extrapolation::Linear);
                let slope = line.end_slope(Side::Above);
                let y = line.lookup(above).unwrap();
                prop_assert_eq!(y.extrapolated, Some(Side::Above));
                prop_assert!((y.value - (ys[n - 1] + slope * beyond)).abs() <= 1e-9 * (1.0 + y.value.abs()));
            }
        }

        #[test]
        fn serde_round_trip_rebuilds_the_table((xs, ys) in knots()) {
            let t = table(&xs, &ys, Interpolation::NaturalCubic, Extrapolation::Linear);
            let json = serde_json::to_string(&t).unwrap();
            let back: Table1D = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back, t);
        }
    }

    #[test]
    fn deserializing_checks_the_knots() {
        let bad = r#"{"x": [0.0, 0.0], "y": [1.0, 2.0]}"#;
        assert!(serde_json::from_str::<Table1D>(bad).is_err());
        let good = r#"{"x": [0.0, 1.0], "y": [1.0, 2.0]}"#;
        let t: Table1D = serde_json::from_str(good).unwrap();
        assert_eq!(t.interpolation(), Interpolation::Linear);
        assert_eq!(t.extrapolation(), Extrapolation::Clamp);
    }
}
