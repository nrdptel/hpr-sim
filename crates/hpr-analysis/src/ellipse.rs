//! Landing ellipses: where a rocket's landings scatter on the ground, as an ellipse that holds a
//! chosen share of them.
//!
//! **Guide:** [Monte Carlo dispersion][guide]'s *Landing ellipses* section draws one from a run
//! and says how far to trust it.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/monte-carlo.html#landing-ellipses
//!
//! A [`Scatter`] keeps the landing points of a run (east and north of the pad, m) and the number
//! of samples tried, so a failed flight, or one that never landed, is counted and not dropped, as
//! in a [`Distribution`](crate::statistics::Distribution).
//!
//! # The ellipse of a normal spread
//!
//! If the landings follow a two-dimensional normal distribution with mean `μ` and covariance `Σ`,
//! the points `x` with `(x − μ)ᵀ Σ⁻¹ (x − μ) ≤ k²` fill an ellipse centred on `μ`. Its axes lie
//! along the eigenvectors of `Σ`, its semi-axes are `k √λ₁` and `k √λ₂` for the eigenvalues
//! `λ₁ ≥ λ₂`, and it holds the probability `P(χ²₂ ≤ k²)`, as the left side is chi-square with two
//! degrees of freedom. That distribution's cumulative function is `1 − e^(−x/2)`, so the ellipse
//! holding a share `p` (its *level*) has
//!
//! `k² = −2 ln(1 − p)`
//!
//! ([`gaussian_scale`]). M. Abramowitz and I. A. Stegun, *Handbook of Mathematical Functions*, NBS
//! AMS 55, 1964, integrate the bivariate normal density over this ellipse to `1 − e^(−k²/2)`
//! (p. 940, eq. 26.3.21), the chi-square function with two degrees of freedom (eq. 26.4.5,
//! p. 941). B. Wang, W. Shi and Z. Miao, "Confidence analysis of standard deviational ellipse and
//! its extension into higher dimensional Euclidean space", *PLoS ONE* 10(3), e0118537, 2015,
//! <https://doi.org/10.1371/journal.pone.0118537>, derive the axes (eqs. 13–16) and the level
//! (eqs. 19–20). For `p` = 50%, 90%, 95% and 99%, `k²` is `2 ln 2`, `2 ln 10`, `2 ln 20` and
//! `4 ln 10`: 1.386, 4.605, 5.991 and 9.210, as the NIST/SEMATECH *e-Handbook of Statistical
//! Methods* tabulates the last three (§1.3.6.7.4,
//! <https://www.itl.nist.gov/div898/handbook/eda/section3/eda3674.htm>).
//!
//! For a symmetric 2 × 2 matrix `Σ = [[a, b], [b, c]]` (`a` the east variance, `c` the north, `b`
//! their covariance) the eigenvalues are `λ = (a + c)/2 ± √(((a − c)/2)² + b²)` and the major
//! axis makes the angle `θ = ½ atan2(2b, a − c)` with east, counter-clockwise. It is reported as
//! a heading, clockwise from north: `π/2 − θ`, in `[0, π)`. A circle (`a = c`, `b = 0`) has no
//! major axis; its heading is reported as east's, `π/2`.
//!
//! [`Scatter::ellipse`] puts the sample's mean and covariance in place of `μ` and `Σ`, its axes
//! measured from the points themselves ([`Scatter::principal_axes`]) so that a very narrow spread
//! keeps its width. The mean
//! and covariance are taken on the points shifted by the first (sorted) one, the covariance with
//! `n − 1` and two passes, as [`Distribution`](crate::statistics::Distribution)'s are (T. F.
//! Chan, G. H. Golub and R. J. LeVeque, *The American Statistician* 37(3), 242–247, 1983).
//!
//! # The ellipse a new flight lands in
//!
//! A sample's mean and covariance are estimates, so the ellipse drawn from them holds a little
//! less than `p` of the flights still to come; with few samples, much less. For normal landings
//! the region a new flight lands in with probability exactly `p`, given `n` flights, is
//! `(x − x̄)ᵀ S⁻¹ (x − x̄) ≤ k²` with
//!
//! `k² = 2 (n + 1)(n − 1) / (n (n − 2)) · F₂,ₙ₋₂(p) = ((n² − 1)/n) ((1 − p)^(−2/(n − 2)) − 1)`
//!
//! ([`prediction_scale`], [`Scatter::prediction_ellipse`]). This follows from the new point
//! `x − x̄` being normal with covariance `(1 + 1/n) Σ` and independent of `S`, so
//! `n/(n + 1) (x − x̄)ᵀ S⁻¹ (x − x̄)` is Hotelling's `T²` with `n − 1` degrees of freedom, which
//! is `2(n − 1)/(n − 2)` times an `F` with 2 and `n − 2` (H. Hotelling, "The generalization of
//! Student's ratio", *Annals of Mathematical Statistics* 2(3), 360–378, 1931, cited for the
//! distribution and not consulted). The formula is checked against the NIST/SEMATECH
//! *e-Handbook of Statistical Methods*, §6.5.4.3.4, which gives the same limit,
//! `p(m + 1)(m − 1)/(m² − mp) F(p, m − p)` for `p` dimensions and `m` points, after T. P. Ryan,
//! *Statistical Methods for Quality Improvement*, 2000, ch. 9
//! (<https://www.itl.nist.gov/div898/handbook/pmc/section5/pmc5434.htm>). The `F` distribution
//! with 2 and `m` degrees of freedom has the cumulative function `1 − (1 + 2f/m)^(−m/2)` (A&S
//! eq. 26.6.4, p. 946), which inverts in closed form. As `n` grows, `k²` falls to the
//! normal ellipse's `−2 ln(1 − p)`: at 200 flights and 95% it is 2.6% above it, and the semi-axes
//! 1.3% longer.
//!
//! # Whether the landings are normal
//!
//! Neither ellipse is right if the landings aren't normal, and they often aren't: a wind whose
//! heading is uncertain spreads them along an arc. [`Scatter::share_inside`] counts the landings
//! an ellipse really holds. A sample that gave no landing could have landed inside or outside,
//! so the share is a [`Share`]: a lower bound counting it outside, an upper bound counting it
//! inside. A share far from the level means the ellipse is the wrong shape for this run; a share
//! close to it is consistent with normal landings, not proof of them. With few landings the share
//! runs high, as the ellipse is fitted to the same points: three points are each exactly
//! `√(4/3)` standard deviations out, so even the 50% ellipse holds all three.
//!
//! Every sum runs over the points sorted (east, then north), so an ellipse is bit for bit the same
//! however the points were computed or ordered.

use serde::{Deserialize, Serialize};

use crate::error::AnalysisError;
use crate::statistics::Share;

/// The scale `k` of the ellipse holding the share `level` of a normal spread whose mean and
/// covariance are known: `k² = −2 ln(1 − level)` (the module's docs).
///
/// # Errors
///
/// [`AnalysisError::Domain`] for a level outside `(0, 1)`.
pub fn gaussian_scale(level: f64) -> Result<f64, AnalysisError> {
    check_level(level)?;
    Ok((-2.0 * (-level).ln_1p()).sqrt())
}

/// The scale `k` of the ellipse a new flight lands in with probability `level`, from `count`
/// normal landings whose mean and covariance were estimated:
/// `k² = ((n² − 1)/n) ((1 − level)^(−2/(n − 2)) − 1)` (the module's docs).
///
/// # Errors
///
/// - [`AnalysisError::Domain`] for a level outside `(0, 1)`.
/// - [`AnalysisError::TooFew`] for fewer than 3 landings, which leave no degrees of freedom.
pub fn prediction_scale(level: f64, count: usize) -> Result<f64, AnalysisError> {
    check_level(level)?;
    if count < 3 {
        return Err(AnalysisError::TooFew {
            what: "landings for a prediction ellipse",
            count,
            minimum: 3,
        });
    }
    // Cast: a count of landings is far below 2⁵³.
    let n = count as f64;
    let power = (-2.0 / (n - 2.0)) * (-level).ln_1p();
    Ok(((n - 1.0) * (n + 1.0) / n * power.exp_m1()).sqrt())
}

/// Refuses a level outside `(0, 1)`, or one that isn't a number.
fn check_level(level: f64) -> Result<(), AnalysisError> {
    if level > 0.0 && level < 1.0 {
        Ok(())
    } else {
        Err(AnalysisError::Domain {
            what: "ellipse level",
            value: level,
        })
    }
}

/// The covariance of a spread of points on the ground, m². It serializes as its three entries and
/// reads back through [`Covariance::new`]'s checks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CovarianceData")]
pub struct Covariance {
    east_m2: f64,
    north_m2: f64,
    east_north_m2: f64,
}

/// The serialized form of a [`Covariance`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CovarianceData {
    east_m2: f64,
    north_m2: f64,
    east_north_m2: f64,
}

impl TryFrom<CovarianceData> for Covariance {
    type Error = AnalysisError;

    fn try_from(data: CovarianceData) -> Result<Self, AnalysisError> {
        Self::new(data.east_m2, data.north_m2, data.east_north_m2)
    }
}

/// The axes of a [`Covariance`]: its eigenvalues, and the heading of the larger one's eigenvector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PrincipalAxes {
    /// The variance along the major axis, the larger eigenvalue `λ₁`, m².
    pub major_variance_m2: f64,
    /// The variance along the minor axis, the smaller eigenvalue `λ₂`, m²; zero for points on a
    /// line.
    pub minor_variance_m2: f64,
    /// The major axis's heading, clockwise from north, in `[0, π)`; east's, `π/2`, for a circle.
    pub major_heading_rad: f64,
}

impl Covariance {
    /// The covariance with the variances `east_m2` and `north_m2` and the covariance
    /// `east_north_m2`.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for an entry that isn't finite, a negative variance, or a
    /// covariance larger than the variances allow (`|east_north| > √east √north`, so the matrix
    /// isn't positive semi-definite), beyond four units of rounding: a covariance of points on a
    /// line, computed in floating point, can pass the bound by that much.
    pub fn new(east_m2: f64, north_m2: f64, east_north_m2: f64) -> Result<Self, AnalysisError> {
        for (what, value) in [
            ("east variance", east_m2),
            ("north variance", north_m2),
            ("east-north covariance", east_north_m2),
        ] {
            if !value.is_finite() {
                return Err(AnalysisError::Domain { what, value });
            }
        }
        for (what, value) in [("east variance", east_m2), ("north variance", north_m2)] {
            if value < 0.0 {
                return Err(AnalysisError::Domain { what, value });
            }
        }
        // Square roots, not squares, so that large entries can't overflow.
        if east_north_m2.abs() > (1.0 + 4.0 * f64::EPSILON) * east_m2.sqrt() * north_m2.sqrt() {
            return Err(AnalysisError::Domain {
                what: "east-north covariance, against the variances",
                value: east_north_m2,
            });
        }
        Ok(Self {
            east_m2,
            north_m2,
            east_north_m2,
        })
    }

    /// The east variance, m².
    pub fn east_m2(&self) -> f64 {
        self.east_m2
    }

    /// The north variance, m².
    pub fn north_m2(&self) -> f64 {
        self.north_m2
    }

    /// The covariance of east and north, m².
    pub fn east_north_m2(&self) -> f64 {
        self.east_north_m2
    }

    /// The eigenvalues and the major axis's heading (the module's docs). The smaller eigenvalue is
    /// taken as `(ac − b²)/λ₁`, the determinant over the larger, rather than
    /// `(a + c)/2 − √(…)`, whose subtraction loses every digit of a spread much narrower than it
    /// is long; it is cut at zero, since rounding can take the determinant just below for points
    /// on a line.
    pub fn principal_axes(&self) -> PrincipalAxes {
        let (a, c, b) = (self.east_m2, self.north_m2, self.east_north_m2);
        let major = (0.5 * a + 0.5 * c) + (0.5 * a - 0.5 * c).hypot(b);
        let determinant = a.mul_add(c, -(b * b));
        // Rounding can put a circle's `a²/a` a unit above `a`: the minor is never the larger.
        let minor = if major > 0.0 {
            (determinant / major).max(0.0).min(major)
        } else {
            0.0
        };
        // Counter-clockwise from east, in [−π/2, π/2].
        let from_east = 0.5 * (2.0 * b).atan2(a - c);
        let heading = std::f64::consts::FRAC_PI_2 - from_east;
        PrincipalAxes {
            major_variance_m2: major,
            minor_variance_m2: minor,
            // `π/2 − θ` lies in [0, π]; π is the same axis as 0 (θ = −π/2, from `atan2(−0, −x)`).
            major_heading_rad: if heading >= std::f64::consts::PI {
                0.0
            } else {
                heading
            },
        }
    }
}

/// An ellipse on the ground, holding a share of the landings. Built by [`Ellipse::gaussian`],
/// [`Scatter::ellipse`] or [`Scatter::prediction_ellipse`]; it serializes as its fields and reads
/// back through checks on each.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EllipseData")]
#[non_exhaustive]
pub struct Ellipse {
    /// Its level, in `(0, 1)`: the share of a normal spread it holds, or for a prediction
    /// ellipse the probability that the next flight lands inside it.
    pub level: f64,
    /// Its scale `k`: the semi-axes are `k` standard deviations along each axis.
    pub scale: f64,
    /// Its centre, east of the pad, m.
    pub centre_east_m: f64,
    /// Its centre, north of the pad, m.
    pub centre_north_m: f64,
    /// Half its long axis, m.
    pub semi_major_m: f64,
    /// Half its short axis, m; zero for landings on a line.
    pub semi_minor_m: f64,
    /// Its long axis's heading, clockwise from north, in `[0, π)`.
    pub major_heading_rad: f64,
}

/// The serialized form of an [`Ellipse`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EllipseData {
    level: f64,
    scale: f64,
    centre_east_m: f64,
    centre_north_m: f64,
    semi_major_m: f64,
    semi_minor_m: f64,
    major_heading_rad: f64,
}

impl TryFrom<EllipseData> for Ellipse {
    type Error = AnalysisError;

    fn try_from(data: EllipseData) -> Result<Self, AnalysisError> {
        check_level(data.level)?;
        for (what, value) in [
            ("ellipse scale", data.scale),
            ("ellipse centre east", data.centre_east_m),
            ("ellipse centre north", data.centre_north_m),
            ("ellipse semi-major axis", data.semi_major_m),
            ("ellipse semi-minor axis", data.semi_minor_m),
            ("ellipse heading", data.major_heading_rad),
        ] {
            if !value.is_finite() {
                return Err(AnalysisError::Domain { what, value });
            }
        }
        if data.scale < 0.0 {
            return Err(AnalysisError::Domain {
                what: "ellipse scale",
                value: data.scale,
            });
        }
        if !(0.0..=data.semi_major_m).contains(&data.semi_minor_m) {
            return Err(AnalysisError::Domain {
                what: "ellipse semi-minor axis, against zero and the semi-major",
                value: data.semi_minor_m,
            });
        }
        if !(0.0..std::f64::consts::PI).contains(&data.major_heading_rad) {
            return Err(AnalysisError::Domain {
                what: "ellipse heading",
                value: data.major_heading_rad,
            });
        }
        Ok(Self {
            level: data.level,
            scale: data.scale,
            centre_east_m: data.centre_east_m,
            centre_north_m: data.centre_north_m,
            semi_major_m: data.semi_major_m,
            semi_minor_m: data.semi_minor_m,
            major_heading_rad: data.major_heading_rad,
        })
    }
}

impl Ellipse {
    /// The ellipse holding the share `level` of a normal spread with the mean
    /// `(centre_east_m, centre_north_m)` and the covariance `covariance`, both known: its scale is
    /// [`gaussian_scale`].
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a level outside `(0, 1)` or a centre that isn't finite.
    pub fn gaussian(
        centre_east_m: f64,
        centre_north_m: f64,
        covariance: &Covariance,
        level: f64,
    ) -> Result<Self, AnalysisError> {
        for (what, value) in [
            ("ellipse centre east", centre_east_m),
            ("ellipse centre north", centre_north_m),
        ] {
            if !value.is_finite() {
                return Err(AnalysisError::Domain { what, value });
            }
        }
        let scale = gaussian_scale(level)?;
        Ok(Self::scaled(
            [centre_east_m, centre_north_m],
            &covariance.principal_axes(),
            level,
            scale,
        ))
    }

    /// The ellipse `scale` standard deviations out along `axes`.
    fn scaled(centre: [f64; 2], axes: &PrincipalAxes, level: f64, scale: f64) -> Self {
        Self {
            level,
            scale,
            centre_east_m: centre[0],
            centre_north_m: centre[1],
            semi_major_m: scale * axes.major_variance_m2.sqrt(),
            semi_minor_m: scale * axes.minor_variance_m2.sqrt(),
            major_heading_rad: axes.major_heading_rad,
        }
    }

    /// Its area, `π a b`, m².
    pub fn area_m2(&self) -> f64 {
        std::f64::consts::PI * self.semi_major_m * self.semi_minor_m
    }

    /// Whether the point `east_m`, `north_m` (m from the pad) lies inside or on the ellipse.
    ///
    /// Neither semi-axis is taken below `10⁻¹²` of the ellipse's size and distance from the pad
    /// (`a + |centre east| + |centre north|`). Rounding in the centre, the heading and this
    /// test's rotation puts a point that lies on a flat ellipse's axis (a zero minor axis, from
    /// landings on a line) a few units of rounding off it, and the floor keeps it inside. Against
    /// any real spread the floor is far below a millimetre.
    pub fn contains(&self, east_m: f64, north_m: f64) -> bool {
        let (east, north) = (east_m - self.centre_east_m, north_m - self.centre_north_m);
        let (sin, cos) = self.major_heading_rad.sin_cos();
        // Along the major axis, whose direction is (sin, cos) in (east, north), and across it.
        let along = east * sin + north * cos;
        let across = east * cos - north * sin;
        let floor =
            1e-12 * (self.semi_major_m + self.centre_east_m.abs() + self.centre_north_m.abs());
        let (a, b) = (self.semi_major_m.max(floor), self.semi_minor_m.max(floor));
        if a > 0.0 && b > 0.0 {
            (along / a).powi(2) + (across / b).powi(2) <= 1.0
        } else {
            // An ellipse of no size at the pad holds the pad alone.
            east == 0.0 && north == 0.0
        }
    }
}

/// Points on the ground (east and north of the pad, m), sorted, and how many samples were tried.
/// It serializes as those two, and reads back through [`Scatter::new`]'s checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ScatterData")]
pub struct Scatter {
    attempted: usize,
    sorted: Vec<[f64; 2]>,
}

/// The serialized form of a [`Scatter`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScatterData {
    attempted: usize,
    sorted: Vec<[f64; 2]>,
}

impl TryFrom<ScatterData> for Scatter {
    type Error = AnalysisError;

    fn try_from(data: ScatterData) -> Result<Self, AnalysisError> {
        Self::new(data.sorted, data.attempted)
    }
}

impl Scatter {
    /// The largest distance east or north of the pad a point may have, m: 10⁹ m, 25 times round
    /// the Earth, and small enough that every square and sum of a scatter stays finite.
    pub const MAX_COORDINATE_M: f64 = 1e9;

    /// The scatter of `points`, each `[east_m, north_m]`, from `attempted` samples (those that
    /// gave no point make up the difference).
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::Domain`] for a coordinate that isn't finite, or is more than
    ///   [`Scatter::MAX_COORDINATE_M`] from the pad.
    /// - [`AnalysisError::Count`] for more points than samples.
    pub fn new(points: Vec<[f64; 2]>, attempted: usize) -> Result<Self, AnalysisError> {
        if points.len() > attempted {
            return Err(AnalysisError::Count {
                what: "points in a scatter, against the samples tried",
                count: points.len(),
                limit: attempted,
            });
        }
        if let Some(&bad) = points
            .iter()
            .flatten()
            .find(|v| v.is_nan() || v.abs() > Self::MAX_COORDINATE_M)
        {
            return Err(AnalysisError::Domain {
                what: "point in a scatter",
                value: bad,
            });
        }
        let mut sorted = points;
        sorted.sort_by(|p, q| p[0].total_cmp(&q[0]).then(p[1].total_cmp(&q[1])));
        Ok(Self { attempted, sorted })
    }

    /// The samples tried.
    pub fn attempted(&self) -> usize {
        self.attempted
    }

    /// The samples that gave a point.
    pub fn count(&self) -> usize {
        self.sorted.len()
    }

    /// The samples that gave no point: failed, or never landed.
    pub fn missing(&self) -> usize {
        self.attempted - self.sorted.len()
    }

    /// The points, sorted by east, then north.
    pub fn points(&self) -> &[[f64; 2]] {
        &self.sorted
    }

    /// The mean point, `x₀ + Σ(xᵢ − x₀)/n` with `x₀` the first; `None` with no points.
    pub fn mean(&self) -> Option<[f64; 2]> {
        let shift = *self.sorted.first()?;
        let mean = self.shifted_mean(shift);
        Some([shift[0] + mean[0], shift[1] + mean[1]])
    }

    /// The mean of the points less `shift`.
    fn shifted_mean(&self, shift: [f64; 2]) -> [f64; 2] {
        let (east, north) = self.sorted.iter().fold((0.0, 0.0), |(east, north), p| {
            (east + (p[0] - shift[0]), north + (p[1] - shift[1]))
        });
        // Cast: a count of points is far below 2⁵³.
        let n = self.sorted.len() as f64;
        [east / n, north / n]
    }

    /// The sample covariance, `Σ(dᵢ − d̄)(dᵢ − d̄)ᵀ/(n − 1)` with `dᵢ = xᵢ − x₀`; `None` with
    /// fewer than two points.
    pub fn covariance(&self) -> Option<Covariance> {
        if self.sorted.len() < 2 {
            return None;
        }
        let shift = *self.sorted.first()?;
        let mean = self.shifted_mean(shift);
        let (mut ee, mut nn, mut en) = (0.0, 0.0, 0.0);
        for p in &self.sorted {
            let east = (p[0] - shift[0]) - mean[0];
            let north = (p[1] - shift[1]) - mean[1];
            ee += east * east;
            nn += north * north;
            en += east * north;
        }
        // Cast: a count of points is far below 2⁵³.
        let dof = (self.sorted.len() - 1) as f64;
        let (east_m2, north_m2) = (ee / dof, nn / dof);
        // The Cauchy–Schwarz bound holds to rounding, which can break it for points on a line
        // (any two points): cut there, so the covariance passes `Covariance::new` and reads back.
        let bound = east_m2.sqrt() * north_m2.sqrt();
        Some(Covariance {
            east_m2,
            north_m2,
            east_north_m2: (en / dof).max(-bound).min(bound),
        })
    }

    /// The ellipse holding the share `level` of a normal spread with this scatter's mean and
    /// covariance ([`gaussian_scale`]); `None` with fewer than two points.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a level outside `(0, 1)`.
    pub fn ellipse(&self, level: f64) -> Result<Option<Ellipse>, AnalysisError> {
        let scale = gaussian_scale(level)?;
        Ok(self.scaled(level, scale))
    }

    /// The ellipse a new flight lands in with probability `level`, if the landings are normal,
    /// allowing for the mean and covariance being estimated from this scatter
    /// ([`prediction_scale`]); `None` with fewer than three points.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a level outside `(0, 1)`.
    pub fn prediction_ellipse(&self, level: f64) -> Result<Option<Ellipse>, AnalysisError> {
        check_level(level)?;
        if self.sorted.len() < 3 {
            return Ok(None);
        }
        let scale = prediction_scale(level, self.sorted.len())?;
        Ok(self.scaled(level, scale))
    }

    /// The ellipse `scale` standard deviations out from the mean; `None` with fewer than two
    /// points.
    fn scaled(&self, level: f64, scale: f64) -> Option<Ellipse> {
        let centre = self.mean()?;
        let axes = self.principal_axes()?;
        Some(Ellipse::scaled(centre, &axes, level, scale))
    }

    /// The scatter's axes: the heading of its [`Covariance::principal_axes`], and the variances
    /// as the mean squares of the points' distances along and across that heading,
    /// `Σ(uᵀdᵢ)²/(n − 1)` with `dᵢ` a point less the mean. With the heading exact these are the
    /// eigenvalues, and an error `δ` in the heading moves them by only `δ² (λ₁ − λ₂)` (they are
    /// Rayleigh quotients). So a spread far narrower than it is long keeps its width, which the
    /// covariance's three entries, each rounded to about `ε λ₁`, can't carry. `None` with fewer
    /// than two points.
    pub fn principal_axes(&self) -> Option<PrincipalAxes> {
        let heading = self.covariance()?.principal_axes().major_heading_rad;
        let (sin, cos) = heading.sin_cos();
        let shift = *self.sorted.first()?;
        let mean = self.shifted_mean(shift);
        let (mut along_squares, mut across_squares) = (0.0, 0.0);
        for p in &self.sorted {
            let east = (p[0] - shift[0]) - mean[0];
            let north = (p[1] - shift[1]) - mean[1];
            along_squares += (east * sin + north * cos).powi(2);
            across_squares += (east * cos - north * sin).powi(2);
        }
        // Cast: a count of points is far below 2⁵³.
        let dof = (self.sorted.len() - 1) as f64;
        let (along, across) = (along_squares / dof, across_squares / dof);
        // Near a circle the heading means nothing and rounding can leave the across spread the
        // larger: then the axis across is the major one.
        Some(if across > along {
            let turned = heading + std::f64::consts::FRAC_PI_2;
            PrincipalAxes {
                major_variance_m2: across,
                minor_variance_m2: along,
                major_heading_rad: if turned >= std::f64::consts::PI {
                    turned - std::f64::consts::PI
                } else {
                    turned
                },
            }
        } else {
            PrincipalAxes {
                major_variance_m2: along,
                minor_variance_m2: across,
                major_heading_rad: heading,
            }
        })
    }

    /// Bounds on the share of the samples tried that landed inside or on `ellipse` ([`Share`]):
    /// a sample with no point counts as outside for `low`, inside for `high`. `None` with no
    /// samples tried.
    pub fn share_inside(&self, ellipse: &Ellipse) -> Option<Share> {
        if self.attempted == 0 {
            return None;
        }
        let inside = self
            .sorted
            .iter()
            .filter(|p| ellipse.contains(p[0], p[1]))
            .count();
        // Cast: counts far below 2⁵³.
        let attempted = self.attempted as f64;
        Some(Share {
            low: inside as f64 / attempted,
            high: (inside + self.missing()) as f64 / attempted,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use hpr_core::random::SeededRng;

    use super::*;

    /// The covariance with the variances `major` and `minor` along axes whose major one has the
    /// heading `heading` (clockwise from north): `R diag(major, minor) Rᵀ` with the major axis's
    /// direction `(sin h, cos h)` in (east, north).
    fn rotated(major: f64, minor: f64, heading: f64) -> Covariance {
        let (s, c) = heading.sin_cos();
        // u = (s, c), v = (c, −s): Σ = major u uᵀ + minor v vᵀ.
        Covariance {
            east_m2: major * s * s + minor * c * c,
            north_m2: major * c * c + minor * s * s,
            east_north_m2: (major - minor) * s * c,
        }
    }

    /// Relative closeness, or absolute near zero.
    fn close(x: f64, y: f64, tolerance: f64) -> bool {
        (x - y).abs() <= tolerance * x.abs().max(y.abs()).max(1.0)
    }

    /// The same heading for an axis, which has no sense: `h` and `h ± π` are one axis.
    fn same_axis(x: f64, y: f64, tolerance: f64) -> bool {
        let d = (x - y).rem_euclid(PI);
        d <= tolerance || PI - d <= tolerance
    }

    #[test]
    fn the_scale_of_a_level_is_the_chi_square_quantile() {
        // χ² with two degrees of freedom at 90%, 95% and 99%, as the NIST/SEMATECH e-Handbook's
        // table prints them (§1.3.6.7.4, three decimals).
        for (level, table) in [(0.9, 4.605), (0.95, 5.991), (0.99, 9.210)] {
            let k = gaussian_scale(level).unwrap();
            assert!((k * k - table).abs() <= 5e-4, "{level}: {}", k * k);
        }
        // To nine figures, the closed forms 2 ln 2, 2 ln 10, 2 ln 20 and 4 ln 10, with ln 2 =
        // 0.693147181, ln 10 = 2.302585093 and ln 20 = 2.995732274.
        for (level, closed) in [
            (0.5, 1.386_294_361),
            (0.9, 4.605_170_186),
            (0.95, 5.991_464_547),
            (0.99, 9.210_340_372),
        ] {
            let k = gaussian_scale(level).unwrap();
            assert!((k * k - closed).abs() < 1e-9, "{level}: {}", k * k);
        }
        // 1 − e^(−1) is exactly the level of k² = 2, up to the level's rounding.
        let k = gaussian_scale(-(-1.0_f64).exp_m1()).unwrap();
        assert!((k * k - 2.0).abs() < 4.0 * f64::EPSILON, "{}", k * k);
    }

    #[test]
    fn principal_axes_of_rotated_covariances() {
        // Every heading of the major axis, round and odd, and every shape: long, round-ish, flat.
        for &(major, minor) in &[(4.0, 1.0), (2500.0, 900.0), (1.0, 0.999), (9.0, 0.0)] {
            for k in 0..24 {
                let heading = f64::from(k) * PI / 24.0 + 0.013;
                let axes = rotated(major, minor, heading).principal_axes();
                assert!(close(axes.major_variance_m2, major, 1e-14), "{axes:?}");
                assert!(
                    (axes.minor_variance_m2 - minor).abs() <= 1e-14 * major,
                    "{axes:?}"
                );
                assert!((0.0..PI).contains(&axes.major_heading_rad), "{axes:?}");
                // The heading is ill-conditioned as the shape nears a circle: its error grows like
                // ε λ₁ / (λ₁ − λ₂).
                let tolerance = 1e-14 * major / (major - minor);
                assert!(
                    same_axis(axes.major_heading_rad, heading, tolerance),
                    "{heading}: {axes:?}"
                );
            }
        }
        // Along the axes, exactly; a circle reports east's heading.
        let east = Covariance::new(4.0, 1.0, 0.0).unwrap().principal_axes();
        assert_eq!(
            (
                east.major_variance_m2,
                east.minor_variance_m2,
                east.major_heading_rad
            ),
            (4.0, 1.0, FRAC_PI_2)
        );
        let north = Covariance::new(1.0, 4.0, 0.0).unwrap().principal_axes();
        assert_eq!(
            (
                north.major_variance_m2,
                north.minor_variance_m2,
                north.major_heading_rad
            ),
            (4.0, 1.0, 0.0)
        );
        let circle = Covariance::new(2.0, 2.0, 0.0).unwrap().principal_axes();
        assert_eq!(
            (
                circle.major_variance_m2,
                circle.minor_variance_m2,
                circle.major_heading_rad
            ),
            (2.0, 2.0, FRAC_PI_2)
        );
        // Equal variances and a positive covariance: the major axis runs north-east.
        let diagonal = Covariance::new(2.0, 2.0, 1.0).unwrap().principal_axes();
        assert_eq!(diagonal.major_heading_rad, FRAC_PI_2 / 2.0);
        assert_eq!(
            (diagonal.major_variance_m2, diagonal.minor_variance_m2),
            (3.0, 1.0)
        );
    }

    /// The probability a normal spread with mean 0 and covariance `sigma` puts inside `ellipse`
    /// (centred at 0), by integrating its density along rays from the centre.
    ///
    /// Along the unit direction `u`, the density is `exp(−r² q/2) / (2π √det Σ)` with
    /// `q = uᵀ Σ⁻¹ u`, so the ray out to the ellipse's edge at `R(u)` carries
    /// `∫₀ᴿ exp(−r² q/2) r dr = (1 − exp(−R² q/2))/q`. The edge comes from the ellipse's own axes
    /// and heading, `Σ⁻¹` from the matrix's entries, so the two meet only if the axes are right.
    /// The integrand is smooth and periodic in the ray's angle, so the trapezoid rule converges
    /// geometrically.
    fn probability_inside(sigma: &Covariance, ellipse: &Ellipse) -> f64 {
        let (a, c, b) = (sigma.east_m2, sigma.north_m2, sigma.east_north_m2);
        let det = a * c - b * b;
        let (inv_ee, inv_nn, inv_en) = (c / det, a / det, -b / det);
        let (sin, cos) = ellipse.major_heading_rad.sin_cos();
        let steps = 4096;
        let mut sum = 0.0;
        for i in 0..steps {
            let phi = 2.0 * PI * f64::from(i) / f64::from(steps);
            let (east, north) = (phi.cos(), phi.sin());
            let q = inv_ee * east * east + 2.0 * inv_en * east * north + inv_nn * north * north;
            let along = east * sin + north * cos;
            let across = east * cos - north * sin;
            let edge_squared = 1.0
                / ((along / ellipse.semi_major_m).powi(2)
                    + (across / ellipse.semi_minor_m).powi(2));
            sum += -(-0.5 * edge_squared * q).exp_m1() / q;
        }
        sum * (2.0 * PI / f64::from(steps)) / (2.0 * PI * det.sqrt())
    }

    #[test]
    fn a_gaussian_ellipse_holds_its_level() {
        for &(major, minor, heading) in &[
            (1.0, 1.0, 0.0),
            (400.0, 100.0, 0.3),
            (40_000.0, 900.0, 2.0),
            (25.0, 24.0, 1.1),
        ] {
            let sigma = rotated(major, minor, heading);
            for level in [0.5, 0.9, 0.95, 0.99] {
                let ellipse = Ellipse::gaussian(0.0, 0.0, &sigma, level).unwrap();
                let p = probability_inside(&sigma, &ellipse);
                assert!((p - level).abs() < 1e-12, "{level}: {p} ({ellipse:?})");
                let k = gaussian_scale(level).unwrap();
                assert!(close(ellipse.semi_major_m, k * major.sqrt(), 1e-14));
                assert!(close(ellipse.semi_minor_m, k * minor.sqrt(), 1e-14));
                assert!(close(
                    ellipse.area_m2(),
                    PI * k * k * (major * minor).sqrt(),
                    1e-14
                ));
            }
        }
        // An ellipse turned 0.1 rad (6°) off the axes holds visibly less: the check can fail. (A
        // turn's loss is of second order, so a much smaller one would hide in the margin.)
        let sigma = rotated(40_000.0, 900.0, 2.0);
        let mut turned = Ellipse::gaussian(0.0, 0.0, &sigma, 0.95).unwrap();
        turned.major_heading_rad += 0.1;
        let p = probability_inside(&sigma, &turned);
        assert!((p - 0.9059).abs() < 1e-4, "{p}");
    }

    #[test]
    fn a_sample_covariance_by_hand() {
        // Four points a distance s₁ either way along a north-east axis and s₂ across it, about
        // (100, −50): Σ(d dᵀ) = 2 s₁² u uᵀ + 2 s₂² v vᵀ over n − 1 = 3.
        let (s1, s2) = (30.0, 10.0);
        let heading = FRAC_PI_2 / 2.0;
        let (sin, cos) = heading.sin_cos();
        let (u, v) = ([sin, cos], [cos, -sin]);
        let centre = [100.0, -50.0];
        let points = [(s1, u), (-s1, u), (s2, v), (-s2, v)]
            .iter()
            .map(|&(s, d)| [centre[0] + s * d[0], centre[1] + s * d[1]])
            .collect();
        let scatter = Scatter::new(points, 4).unwrap();
        let mean = scatter.mean().unwrap();
        assert!(
            close(mean[0], 100.0, 1e-15) && close(mean[1], -50.0, 1e-15),
            "{mean:?}"
        );
        let axes = scatter.covariance().unwrap().principal_axes();
        assert!(
            close(axes.major_variance_m2, 2.0 * s1 * s1 / 3.0, 1e-14),
            "{axes:?}"
        );
        assert!(
            close(axes.minor_variance_m2, 2.0 * s2 * s2 / 3.0, 1e-13),
            "{axes:?}"
        );
        assert!(
            same_axis(axes.major_heading_rad, heading, 1e-14),
            "{axes:?}"
        );
        // The prediction ellipse is wider than the normal one at the same level.
        let normal = scatter.ellipse(0.95).unwrap().unwrap();
        let prediction = scatter.prediction_ellipse(0.95).unwrap().unwrap();
        assert!(prediction.semi_major_m > normal.semi_major_m);
        assert_eq!(prediction.scale, prediction_scale(0.95, 4).unwrap());
    }

    /// `count` points from the normal spread with mean `centre` and covariance `sigma`, by its
    /// Cholesky factor `L` (`Σ = L Lᵀ`) times standard normal pairs.
    fn normal_points(
        rng: &mut SeededRng,
        centre: [f64; 2],
        sigma: &Covariance,
        count: usize,
    ) -> Vec<[f64; 2]> {
        let l11 = sigma.east_m2.sqrt();
        let l21 = sigma.east_north_m2 / l11;
        let l22 = (sigma.north_m2 - l21 * l21).sqrt();
        (0..count)
            .map(|_| {
                let (z1, z2) = (rng.standard_normal(), rng.standard_normal());
                [centre[0] + l11 * z1, centre[1] + l21 * z1 + l22 * z2]
            })
            .collect()
    }

    #[test]
    fn a_sampled_gaussian_gives_back_its_ellipse() {
        // 100,000 landings from a known normal spread: the covariance and the share inside each
        // ellipse within five standard errors of the truth.
        let n = 100_000;
        let sigma = rotated(250_000.0, 40_000.0, 1.2);
        let centre = [600.0, -150.0];
        let mut rng = SeededRng::seed_from_u64(61);
        let scatter = Scatter::new(normal_points(&mut rng, centre, &sigma, n), n).unwrap();
        let estimate = scatter.covariance().unwrap();
        let dof = (n - 1) as f64;
        // A sample (co)variance's standard error: √((σᵢⱼ² + σᵢᵢ σⱼⱼ)/(n − 1)).
        let error = |sij: f64, sii: f64, sjj: f64| ((sij * sij + sii * sjj) / dof).sqrt();
        let (a, c, b) = (sigma.east_m2, sigma.north_m2, sigma.east_north_m2);
        assert!(
            (estimate.east_m2 - a).abs() < 5.0 * error(a, a, a),
            "{estimate:?}"
        );
        assert!(
            (estimate.north_m2 - c).abs() < 5.0 * error(c, c, c),
            "{estimate:?}"
        );
        assert!(
            (estimate.east_north_m2 - b).abs() < 5.0 * error(b, a, c),
            "{estimate:?}"
        );
        let mean = scatter.mean().unwrap();
        assert!(
            (mean[0] - centre[0]).abs() < 5.0 * (a / n as f64).sqrt(),
            "{mean:?}"
        );
        assert!(
            (mean[1] - centre[1]).abs() < 5.0 * (c / n as f64).sqrt(),
            "{mean:?}"
        );
        for level in [0.5, 0.9, 0.95, 0.99] {
            // The share of the sample inside its own ellipse, and inside the true one.
            let binomial = (level * (1.0 - level) / n as f64).sqrt();
            let own = scatter.ellipse(level).unwrap().unwrap();
            let share = scatter.share_inside(&own).unwrap();
            assert_eq!(share.low, share.high);
            assert!(
                (share.low - level).abs() < 5.0 * binomial,
                "{level}: {share:?}"
            );
            let truth = Ellipse::gaussian(centre[0], centre[1], &sigma, level).unwrap();
            let share = scatter.share_inside(&truth).unwrap();
            assert!(
                (share.low - level).abs() < 5.0 * binomial,
                "{level}: {share:?}"
            );
        }
    }

    #[test]
    fn a_prediction_ellipse_holds_a_new_flight_at_its_level() {
        // Many runs of a few flights each, and one more flight: the new one lands inside the
        // prediction ellipse drawn from the few at its level, within five standard errors, and
        // inside the normal one visibly less often.
        let sigma = rotated(900.0, 100.0, 0.7);
        let level = 0.9;
        let trials = 20_000;
        let binomial = (level * (1.0 - level) / f64::from(trials)).sqrt();
        for count in [3, 5, 20] {
            let mut rng = SeededRng::for_stream(62, &[count as u64]);
            let (mut predicted, mut normal) = (0_u32, 0_u32);
            for _ in 0..trials {
                let mut points = normal_points(&mut rng, [0.0, 0.0], &sigma, count + 1);
                let new = points.pop().unwrap();
                let scatter = Scatter::new(points, count).unwrap();
                let wide = scatter.prediction_ellipse(level).unwrap().unwrap();
                predicted += u32::from(wide.contains(new[0], new[1]));
                let narrow = scatter.ellipse(level).unwrap().unwrap();
                normal += u32::from(narrow.contains(new[0], new[1]));
            }
            let share = f64::from(predicted) / f64::from(trials);
            assert!((share - level).abs() < 5.0 * binomial, "{count}: {share}");
            let short = f64::from(normal) / f64::from(trials);
            assert!(short < level - 10.0 * binomial, "{count}: {short}");
        }
    }

    #[test]
    fn the_prediction_scale_by_hand_and_in_the_limit() {
        // n = 5 at 95%: k² = 2·6·4/(5·3) F₂,₃(0.95), with F₂,₃(0.95) = 1.5 (0.05^(−2/3) − 1) =
        // 9.552, an F table's 9.55.
        let k = prediction_scale(0.95, 5).unwrap();
        let f = 1.5 * (0.05_f64.powf(-2.0 / 3.0) - 1.0);
        assert!((f - 9.552_094).abs() < 1e-6, "{f}");
        assert!(
            close(k * k, 2.0 * 6.0 * 4.0 / (5.0 * 3.0) * f, 1e-14),
            "{}",
            k * k
        );
        // As the flights grow, it falls to the normal ellipse's.
        let normal = gaussian_scale(0.95).unwrap();
        let mut last = f64::INFINITY;
        for count in [3, 10, 100, 1000, 10_000, 1_000_000] {
            let k = prediction_scale(0.95, count).unwrap();
            assert!(k < last && k > normal, "{count}: {k}");
            last = k;
        }
        // The excess falls like 1/n.
        assert!(close(last, normal, 1e-5), "{last}");
        let at_200 = prediction_scale(0.95, 200).unwrap().powi(2) / (normal * normal);
        // 6.1443 against 5.9915: 2.55% more, the semi-axes 1.3% longer.
        assert!((at_200 - 1.025_513).abs() < 1e-6, "{at_200}");
    }

    #[test]
    fn a_narrow_spread_keeps_its_width() {
        // A spread 10⁸ times longer than it is wide, at an odd heading: the minor variance is
        // 10⁻¹² m², 10⁻¹⁶ of the major, below the rounding of the covariance's entries, from which
        // any formula gave 0 and an ellipse holding almost none of the landings.
        // The points are drawn along the axes, `μ + z₁ √λ₁ u + z₂ √λ₂ v`: a Cholesky factor's
        // `√(c − l₂₁²)` would cancel to noise at this width.
        let n = 10_000;
        let (sin, cos) = 1.0_f64.sin_cos();
        let mut rng = SeededRng::seed_from_u64(64);
        let points = (0..n)
            .map(|_| {
                let (along, across) = (1e2 * rng.standard_normal(), 1e-6 * rng.standard_normal());
                [
                    300.0 + along * sin + across * cos,
                    40.0 + along * cos - across * sin,
                ]
            })
            .collect();
        let scatter = Scatter::new(points, n).unwrap();
        let axes = scatter.principal_axes().unwrap();
        let error = |s: f64| s * (2.0 / (n - 1) as f64).sqrt();
        assert!(
            (axes.minor_variance_m2 - 1e-12).abs() < 5.0 * error(1e-12),
            "{axes:?}"
        );
        for level in [0.5, 0.95] {
            let binomial = (level * (1.0 - level) / n as f64).sqrt();
            let ellipse = scatter.ellipse(level).unwrap().unwrap();
            let share = scatter.share_inside(&ellipse).unwrap();
            assert!(
                (share.low - level).abs() < 5.0 * binomial,
                "{level}: {share:?}"
            );
        }
    }

    #[test]
    fn two_landings_and_a_slanted_line() {
        // Any two landings lie on a line. Their covariance must pass its own checks and read
        // back, and their ellipses hold both: each is √½ standard deviations from the mean.
        let mut rng = SeededRng::seed_from_u64(65);
        for _ in 0..1000 {
            let mut point = || {
                [
                    1000.0 * rng.uniform() - 500.0,
                    1000.0 * rng.uniform() - 500.0,
                ]
            };
            let points = vec![point(), point()];
            let scatter = Scatter::new(points.clone(), 2).unwrap();
            let covariance = scatter.covariance().unwrap();
            let again = Covariance::new(
                covariance.east_m2(),
                covariance.north_m2(),
                covariance.east_north_m2(),
            )
            .unwrap();
            assert_eq!(again, covariance);
            let json = serde_json::to_string(&covariance).unwrap();
            assert_eq!(
                serde_json::from_str::<Covariance>(&json).unwrap(),
                covariance
            );
            let ellipse = scatter.ellipse(0.5).unwrap().unwrap();
            for p in &points {
                assert!(ellipse.contains(p[0], p[1]), "{points:?}: {ellipse:?}");
            }
            let json = serde_json::to_string(&ellipse).unwrap();
            assert_eq!(serde_json::from_str::<Ellipse>(&json).unwrap(), ellipse);
        }
        // Fifty landings evenly along a line at 30° from north: the 95% ellipse holds every one,
        // and nothing a micrometre off the line.
        let (sin, cos) = (PI / 6.0).sin_cos();
        let line: Vec<[f64; 2]> = (1..=50)
            .map(|t| [f64::from(t) * 10.0 * sin, f64::from(t) * 10.0 * cos])
            .collect();
        let scatter = Scatter::new(line, 50).unwrap();
        let ellipse = scatter.ellipse(0.95).unwrap().unwrap();
        assert!(
            same_axis(ellipse.major_heading_rad, PI / 6.0, 1e-14),
            "{ellipse:?}"
        );
        assert_eq!(
            scatter.share_inside(&ellipse).unwrap().low,
            1.0,
            "{ellipse:?}"
        );
        assert!(!ellipse.contains(250.0 * sin + 1e-6 * cos, 250.0 * cos - 1e-6 * sin));
        // A rank-one covariance computed in floating point passes its checks at every heading.
        for k in 0..24 {
            let flat = rotated(9.0, 0.0, f64::from(k) * PI / 24.0 + 0.013);
            Covariance::new(flat.east_m2, flat.north_m2, flat.east_north_m2).unwrap();
        }
    }

    #[test]
    fn a_circle_s_axes_stay_ordered() {
        // `a²/a` rounds a unit above `a` for some `a`: the minor axis must still not pass the
        // major, or the ellipse fails its own checks on reading back.
        let mut rng = SeededRng::seed_from_u64(66);
        for _ in 0..10_000 {
            let a = 1.0 + 1000.0 * rng.uniform();
            for b in [0.0, 1e-13 * a] {
                let sigma = Covariance::new(a, a, b).unwrap();
                let axes = sigma.principal_axes();
                assert!(
                    axes.minor_variance_m2 <= axes.major_variance_m2,
                    "{a}: {axes:?}"
                );
                let ellipse = Ellipse::gaussian(0.0, 0.0, &sigma, 0.9).unwrap();
                let json = serde_json::to_string(&ellipse).unwrap();
                assert_eq!(serde_json::from_str::<Ellipse>(&json).unwrap(), ellipse);
            }
        }
    }

    #[test]
    fn an_ellipse_reads_back_through_its_checks() {
        let sigma = Covariance::new(4.0, 1.0, 0.5).unwrap();
        let ellipse = Ellipse::gaussian(10.0, -20.0, &sigma, 0.9).unwrap();
        let json = serde_json::to_string(&ellipse).unwrap();
        assert_eq!(serde_json::from_str::<Ellipse>(&json).unwrap(), ellipse);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        for (field, bad, what) in [
            ("level", 1.5, "ellipse level"),
            ("scale", -1.0, "ellipse scale"),
            (
                "semi_minor_m",
                1e3,
                "ellipse semi-minor axis, against zero and the semi-major",
            ),
            (
                "semi_minor_m",
                -1.0,
                "ellipse semi-minor axis, against zero and the semi-major",
            ),
            ("major_heading_rad", PI, "ellipse heading"),
        ] {
            let mut edited = value.clone();
            edited[field] = serde_json::json!(bad);
            let error = serde_json::from_value::<Ellipse>(edited).unwrap_err();
            assert!(error.to_string().contains(what), "{field}: {error}");
        }
    }

    proptest::proptest! {
        /// Whatever the points: the covariance passes its own checks, the heading lies in
        /// [0, π), and for 2 to 20 points the 99.99% ellipse holds every one of them. A sample
        /// point is at most (n − 1)/√n standard deviations from the mean (its leverage is at most
        /// 1), which for 20 points is √18.05, inside the 99.99% ellipse's √18.42.
        #[test]
        fn every_point_lies_in_its_own_wide_ellipse(
            points in proptest::collection::vec((-1e4..1e4_f64, -1e4..1e4_f64), 2..=20),
        ) {
            let points: Vec<[f64; 2]> = points.into_iter().map(|(e, n)| [e, n]).collect();
            let count = points.len();
            let scatter = Scatter::new(points.clone(), count).unwrap();
            let c = scatter.covariance().unwrap();
            proptest::prop_assert!(Covariance::new(c.east_m2(), c.north_m2(), c.east_north_m2()).is_ok());
            let ellipse = scatter.ellipse(0.9999).unwrap().unwrap();
            proptest::prop_assert!((0.0..PI).contains(&ellipse.major_heading_rad));
            for p in &points {
                proptest::prop_assert!(ellipse.contains(p[0], p[1]), "{:?}: {:?}", p, ellipse);
            }
        }
    }

    #[test]
    fn landings_on_a_line_and_on_a_point() {
        // Due north of the pad: a flat ellipse holding its own line.
        let line = Scatter::new(vec![[0.0, 10.0], [0.0, 20.0], [0.0, 30.0]], 3).unwrap();
        let ellipse = line.ellipse(0.95).unwrap().unwrap();
        assert_eq!(
            (ellipse.semi_minor_m, ellipse.major_heading_rad),
            (0.0, 0.0)
        );
        assert!(ellipse.contains(0.0, 25.0) && !ellipse.contains(0.1, 25.0));
        assert!(!ellipse.contains(0.0, 20.0 + 1.01 * ellipse.semi_major_m));
        assert_eq!(line.share_inside(&ellipse).unwrap().low, 1.0);
        // Every flight on one spot: an ellipse of no size, holding that spot alone.
        let spot = Scatter::new(vec![[5.0, 5.0]; 4], 4).unwrap();
        let ellipse = spot.ellipse(0.5).unwrap().unwrap();
        assert_eq!((ellipse.semi_major_m, ellipse.semi_minor_m), (0.0, 0.0));
        assert_eq!((ellipse.centre_east_m, ellipse.centre_north_m), (5.0, 5.0));
        assert!(ellipse.contains(5.0, 5.0) && !ellipse.contains(5.0, 5.0 + 1e-9));
        // Too few points: no covariance, or no prediction.
        let one = Scatter::new(vec![[1.0, 2.0]], 3).unwrap();
        assert_eq!(one.mean(), Some([1.0, 2.0]));
        assert_eq!((one.covariance(), one.ellipse(0.5).unwrap()), (None, None));
        let two = Scatter::new(vec![[1.0, 2.0], [3.0, 4.0]], 2).unwrap();
        assert!(two.ellipse(0.5).unwrap().is_some());
        assert_eq!(two.prediction_ellipse(0.5).unwrap(), None);
        let none = Scatter::new(vec![], 0).unwrap();
        assert_eq!((none.mean(), none.share_inside(&ellipse)), (None, None));
    }

    #[test]
    fn missing_landings_bound_the_share() {
        // Two of six flights gave no landing: between 4/6 and 6/6 of them landed inside.
        let scatter =
            Scatter::new(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], 6).unwrap();
        assert_eq!(
            (scatter.count(), scatter.missing(), scatter.attempted()),
            (4, 2, 6)
        );
        let ellipse = scatter.ellipse(0.99).unwrap().unwrap();
        let share = scatter.share_inside(&ellipse).unwrap();
        assert_eq!((share.low, share.high), (4.0 / 6.0, 1.0));
    }

    #[test]
    fn the_order_of_the_points_changes_nothing() {
        let sigma = rotated(400.0, 100.0, 0.4);
        let mut rng = SeededRng::seed_from_u64(63);
        let points = normal_points(&mut rng, [10.0, 20.0], &sigma, 1000);
        let mut reversed = points.clone();
        reversed.reverse();
        let (forward, backward) = (
            Scatter::new(points, 1000).unwrap(),
            Scatter::new(reversed, 1000).unwrap(),
        );
        assert_eq!(forward, backward);
        let (x, y) = (
            forward.ellipse(0.95).unwrap().unwrap(),
            backward.ellipse(0.95).unwrap().unwrap(),
        );
        assert_eq!(x.semi_major_m.to_bits(), y.semi_major_m.to_bits());
        assert_eq!(x.major_heading_rad.to_bits(), y.major_heading_rad.to_bits());
    }

    #[test]
    fn a_scatter_and_a_covariance_read_back_through_their_checks() {
        let scatter = Scatter::new(vec![[3.0, 1.0], [1.0, 2.0]], 3).unwrap();
        let json = serde_json::to_string(&scatter).unwrap();
        assert_eq!(json, r#"{"attempted":3,"sorted":[[1.0,2.0],[3.0,1.0]]}"#);
        assert_eq!(serde_json::from_str::<Scatter>(&json).unwrap(), scatter);
        let error =
            serde_json::from_str::<Scatter>(r#"{"attempted":1,"sorted":[[1.0,2.0],[3.0,1.0]]}"#)
                .unwrap_err();
        assert!(error.to_string().contains("more than 1"), "{error}");
        let covariance = Covariance::new(4.0, 1.0, 1.5).unwrap();
        let json = serde_json::to_string(&covariance).unwrap();
        assert_eq!(
            json,
            r#"{"east_m2":4.0,"north_m2":1.0,"east_north_m2":1.5}"#
        );
        assert_eq!(
            serde_json::from_str::<Covariance>(&json).unwrap(),
            covariance
        );
        let error = serde_json::from_str::<Covariance>(
            r#"{"east_m2":4.0,"north_m2":1.0,"east_north_m2":2.5}"#,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("against the variances"),
            "{error}"
        );
    }

    #[test]
    fn bad_inputs_are_refused() {
        assert!(matches!(
            Scatter::new(vec![[1.0, f64::NAN]], 1),
            Err(AnalysisError::Domain { what: "point in a scatter", value }) if value.is_nan()
        ));
        // Past 10⁹ m the squares could overflow; at it, they don't.
        assert!(matches!(
            Scatter::new(vec![[1e200, 0.0], [-1e200, 0.0]], 2),
            Err(AnalysisError::Domain { what: "point in a scatter", value }) if value == 1e200
        ));
        let far = Scatter::new(vec![[1e9, -1e9], [-1e9, 1e9], [1e9, 1e9]], 3).unwrap();
        assert!(far.ellipse(0.5).unwrap().is_some());
        assert!(matches!(
            Scatter::new(vec![[1.0, 2.0], [3.0, 4.0]], 1),
            Err(AnalysisError::Count {
                count: 2,
                limit: 1,
                ..
            })
        ));
        let scatter = Scatter::new(vec![[1.0, 2.0], [3.0, 5.0], [4.0, 4.0]], 3).unwrap();
        for level in [0.0, 1.0, -0.1, 1.5, f64::NAN, f64::INFINITY] {
            for result in [
                scatter.ellipse(level),
                scatter.prediction_ellipse(level),
                gaussian_scale(level).map(|_| None),
                prediction_scale(level, 10).map(|_| None),
            ] {
                assert!(matches!(
                    result,
                    Err(AnalysisError::Domain { what: "ellipse level", value })
                        if value.to_bits() == level.to_bits()
                ));
            }
        }
        let error = prediction_scale(0.5, 2).unwrap_err();
        assert!(matches!(
            error,
            AnalysisError::TooFew {
                what: "landings for a prediction ellipse",
                count: 2,
                minimum: 3,
            }
        ));
        assert_eq!(
            error.to_string(),
            "landings for a prediction ellipse: 2 given, at least 3 needed"
        );
        assert!(matches!(
            Covariance::new(-1.0, 1.0, 0.0),
            Err(AnalysisError::Domain { what: "east variance", value }) if value == -1.0
        ));
        assert!(matches!(
            Covariance::new(1.0, -1.0, 0.0),
            Err(AnalysisError::Domain { what: "north variance", value }) if value == -1.0
        ));
        assert!(matches!(
            Covariance::new(1.0, 1.0, f64::INFINITY),
            Err(AnalysisError::Domain {
                what: "east-north covariance",
                ..
            })
        ));
        assert!(matches!(
            Covariance::new(1.0, 4.0, -2.5),
            Err(AnalysisError::Domain {
                what: "east-north covariance, against the variances",
                value
            }) if value == -2.5
        ));
        let sigma = Covariance::new(1.0, 1.0, 0.0).unwrap();
        assert!(matches!(
            Ellipse::gaussian(f64::NAN, 0.0, &sigma, 0.5),
            Err(AnalysisError::Domain {
                what: "ellipse centre east",
                ..
            })
        ));
        assert!(matches!(
            Ellipse::gaussian(0.0, f64::INFINITY, &sigma, 0.5),
            Err(AnalysisError::Domain {
                what: "ellipse centre north",
                ..
            })
        ));
    }
}
