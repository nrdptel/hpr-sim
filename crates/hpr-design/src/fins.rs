//! Fin sets and tube fins: planforms, cross-sections, tabs, and mass properties from geometry.
//!
//! **Fin frame.** A fin set's frame has its origin on the body axis at the station of the root
//! chord's leading edge. Fin `k` of `N` lies in the plane through the axis at roll angle
//! `φ_k = φ_0 + 2πk/N` from `x_B` toward `y_B`. In a fin's own coordinates, `x` runs aft from the
//! root leading edge, the span `h` runs outward from the body surface (radius `R_b + h`), and the
//! thickness coordinate `τ` is normal to the fin plane. Planform definitions follow S. Niskanen,
//! *OpenRocket technical documentation* v13.05, §3.2.2, pp. 25–29: a trapezoidal fin has root chord
//! `c_r`, tip chord `c_t` parallel to the body, span `s`, and sweep `x_t`, the axial distance from
//! the root leading edge to the tip leading edge; an elliptical fin has chord
//! `c(h) = c_r √(1 − (h/s)²)` centred on the root chord; a freeform fin is a polygon.
//!
//! **Mass.** Each fin is a plate whose local thickness varies along each chord with the
//! cross-section. At span `h`, a chord from `a` to `b` (length `c`) contributes the moments
//! `M_k = ∫ x^k t(x) dx` and `T = ∫ t(x)³/12 dx`:
//!
//! ```text
//! square:   t(x) = t
//! rounded:  semicircular leading and trailing edges of radius a_r = min(t, c)/2
//! airfoil:  t(x) = 10 t P(ξ),  P = 0.2969√ξ − 0.1260ξ − 0.3516ξ² + 0.2843ξ³ − 0.1015ξ⁴
//! ```
//!
//! The airfoil is the NACA four-digit symmetric thickness distribution (I. H. Abbott and A. E.
//! von Doenhoff, *Theory of Wing Sections*, Dover, 1959, eq. 6.2) with its maximum thickness,
//! `1.0003 t` at `ξ = 0.2998`, scaled to the fin thickness. OpenRocket's documentation uses the
//! cross-section only for drag (§3.4.4, pp. 49–50); hpr-design also counts the volume it removes.
//! The fin's moments about the fin-set frame are then
//!
//! ```text
//! m = ρ ∫ M_0 dh,  ∫ r dm = ρ ∫ (R_b + h) M_0 dh,  ∫ r² dm = ρ ∫ (R_b + h)² M_0 dh,
//! ∫ x dm = ρ ∫ M_1 dh,  ∫ x² dm = ρ ∫ M_2 dh,  ∫ r x dm = ρ ∫ (R_b + h) M_1 dh,  ∫ τ² dm = ρ ∫ T dh
//! ```
//!
//! and with the fin at `φ = 0` (points at `(r, τ, −x)` in body axes)
//!
//! ```text
//! I_xx = ∫(τ² + x²) dm,  I_yy = ∫(r² + x²) dm,  I_zz = ∫(r² + τ²) dm,  I_xz = ∫ r x dm,  I_xy = I_yz = 0
//! ```
//!
//! about the origin. A tab is a square-section slab below the root (`−h_tab ≤ h ≤ 0`). The flat
//! root is taken to sit on the body at radius `R_b`; the sliver between a flat root and the curved
//! tube, `t²/8R_b` deep, is ignored. **Cant** `δ` turns each fin (with its tab) by `δ` about its own
//! outward span axis through the root mid-chord, right-handed, so a positive cant turns fin 0's
//! leading edge toward `+y_B`. See `docs/physics/mass.md`.

use std::f64::consts::PI;

use hpr_core::quadrature::{Tolerance, integrate};
use hpr_core::{DMat3, DQuat, DVec3};
use serde::{Deserialize, Serialize};

use crate::error::DesignError;
use crate::mass::MassProperties;
use crate::material::Material;
use crate::shapes::check_dimension;

/// The outline of one fin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum FinPlanform {
    /// A trapezoid with its tip chord parallel to the root.
    Trapezoidal {
        /// Root chord, m.
        root_chord_m: f64,
        /// Tip chord, m (zero for a pointed fin).
        tip_chord_m: f64,
        /// Span from the body surface to the tip, m.
        span_m: f64,
        /// Axial distance from the root leading edge aft to the tip leading edge, m.
        sweep_m: f64,
    },
    /// Half an ellipse on the root chord.
    Elliptical {
        /// Root chord, m.
        root_chord_m: f64,
        /// Span, m.
        span_m: f64,
    },
    /// A polygon given as `[x, h]` points from the root leading edge around to the root trailing
    /// edge, closed along the root (`h = 0`); `x` runs aft and `h` outward, in metres.
    Freeform {
        /// The outline, m.
        points_m: Vec<[f64; 2]>,
    },
}

/// The shape of a fin's section along its chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FinCrossSection {
    /// Constant thickness, square edges.
    #[default]
    Square,
    /// Semicircular leading and trailing edges.
    Rounded,
    /// The NACA four-digit symmetric thickness distribution.
    Airfoil,
}

/// A rectangular tab below a fin's root, reaching into the body.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinTab {
    /// Depth below the root, m.
    pub height_m: f64,
    /// Length along the root, m.
    pub length_m: f64,
    /// Distance from the root leading edge aft to the tab's leading edge, m.
    pub offset_m: f64,
}

/// A set of identical fins spaced evenly around the body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinSet {
    /// Number of fins, at least 1.
    pub count: u32,
    /// Outline of each fin.
    pub planform: FinPlanform,
    /// Maximum thickness, m.
    pub thickness_m: f64,
    /// Section shape.
    #[serde(default)]
    pub cross_section: FinCrossSection,
    /// Optional tab below each root.
    #[serde(default)]
    pub tab: Option<FinTab>,
    /// Cant angle, rad.
    #[serde(default)]
    pub cant_rad: f64,
    /// Roll angle of the first fin from `x_B` toward `y_B`, rad.
    #[serde(default)]
    pub base_angle_rad: f64,
    /// Material (bulk).
    pub material: Material,
}

/// Area and centroid of a planform.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlanformGeometry {
    /// Area of one fin, m².
    pub area_m2: f64,
    /// Centroid, m aft of the root leading edge.
    pub centroid_x_m: f64,
    /// Centroid, m outward from the root.
    pub centroid_span_m: f64,
}

/// NACA four-digit thickness polynomial `P(ξ)` (Abbott and von Doenhoff, eq. 6.2); the mass code
/// uses its moments, and the tests integrate it directly.
#[cfg(test)]
fn naca_polynomial(xi: f64) -> f64 {
    0.2969 * xi.sqrt() - 0.1260 * xi - 0.3516 * xi * xi + 0.2843 * xi.powi(3) - 0.1015 * xi.powi(4)
}

/// `10 ∫₀¹ P dξ`, `10 ∫₀¹ ξ P dξ` and `10 ∫₀¹ ξ² P dξ`, term by term.
const AIRFOIL_PHI: [f64; 3] = [
    10.0 * (0.2969 * 2.0 / 3.0 - 0.1260 / 2.0 - 0.3516 / 3.0 + 0.2843 / 4.0 - 0.1015 / 5.0),
    10.0 * (0.2969 * 2.0 / 5.0 - 0.1260 / 3.0 - 0.3516 / 4.0 + 0.2843 / 5.0 - 0.1015 / 6.0),
    10.0 * (0.2969 * 2.0 / 7.0 - 0.1260 / 4.0 - 0.3516 / 5.0 + 0.2843 / 6.0 - 0.1015 / 7.0),
];

/// `1000 ∫₀¹ P³ dξ`, evaluated with mpmath at 30 digits; `airfoil_constants_are_the_integrals`
/// recomputes it.
const AIRFOIL_PSI: f64 = 0.472_889_488_894_451_523_708_489_652_762;

/// Chordwise moments `[M_0, M_1, M_2, T]` of a chord from `a` to `b` of thickness `t`.
fn chord_moments(section: FinCrossSection, a: f64, b: f64, t: f64) -> [f64; 4] {
    let c = b - a;
    if c <= 0.0 {
        return [0.0; 4];
    }
    match section {
        FinCrossSection::Square => [
            t * c,
            0.5 * t * (b * b - a * a),
            t * (b.powi(3) - a.powi(3)) / 3.0,
            t.powi(3) * c / 12.0,
        ],
        FinCrossSection::Airfoil => {
            let [p0, p1, p2] = AIRFOIL_PHI;
            [
                t * c * p0,
                t * (a * c * p0 + c * c * p1),
                t * (a * a * c * p0 + 2.0 * a * c * c * p1 + c.powi(3) * p2),
                t.powi(3) * c * AIRFOIL_PSI / 12.0,
            ]
        }
        FinCrossSection::Rounded => {
            // A slab of the middle thickness minus what the two rounded edges remove. With
            // v = a_r − u the depth from the edge's centre, the removed thickness is
            // δ = 2a_r − 2√(a_r² − v²), whose moments from the edge are D_0, D_1, D_2.
            let mid = t.min(c);
            let r = 0.5 * mid;
            let d0 = r * r * (2.0 - PI / 2.0);
            let d1 = r * d0 - r.powi(3) / 3.0;
            let d2 = r * r * d0 - PI * r.powi(4) / 8.0;
            let e0 = 8.0 * r.powi(4) - 1.5 * PI * r.powi(4);
            let m0 = mid * c - 2.0 * d0;
            [
                m0,
                m0 * 0.5 * (a + b),
                mid * (b.powi(3) - a.powi(3)) / 3.0
                    - (a * a * d0 + 2.0 * a * d1 + d2)
                    - (b * b * d0 - 2.0 * b * d1 + d2),
                (mid.powi(3) * c - 2.0 * e0) / 12.0,
            ]
        }
    }
}

impl FinPlanform {
    /// Root chord, m.
    pub fn root_chord_m(&self) -> f64 {
        match self {
            Self::Trapezoidal { root_chord_m, .. } | Self::Elliptical { root_chord_m, .. } => {
                *root_chord_m
            }
            Self::Freeform { points_m } => {
                let first = points_m.first().map_or(0.0, |p| p[0]);
                let last = points_m.last().map_or(0.0, |p| p[0]);
                (last - first).abs()
            }
        }
    }

    /// Span, m.
    pub fn span_m(&self) -> f64 {
        match self {
            Self::Trapezoidal { span_m, .. } | Self::Elliptical { span_m, .. } => *span_m,
            Self::Freeform { points_m } => points_m.iter().fold(0.0, |m, p| m.max(p[1])),
        }
    }

    /// Checks dimensions, and that a freeform outline is a simple polygon above the root that
    /// starts and ends on it.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] or [`DesignError::Geometry`].
    pub fn validate(&self) -> Result<(), DesignError> {
        match self {
            Self::Trapezoidal {
                root_chord_m,
                tip_chord_m,
                span_m,
                sweep_m,
            } => {
                check_dimension("fin root chord", *root_chord_m, false)?;
                check_dimension("fin tip chord", *tip_chord_m, true)?;
                check_dimension("fin span", *span_m, false)?;
                if !sweep_m.is_finite() {
                    return Err(DesignError::Domain {
                        what: "fin sweep",
                        value: *sweep_m,
                    });
                }
            }
            Self::Elliptical {
                root_chord_m,
                span_m,
            } => {
                check_dimension("fin root chord", *root_chord_m, false)?;
                check_dimension("fin span", *span_m, false)?;
            }
            Self::Freeform { points_m } => validate_outline(points_m)?,
        }
        Ok(())
    }

    /// The chord intervals `[a, b]` crossing span `h`, sorted, into `out`.
    fn chords(&self, h: f64, out: &mut Vec<(f64, f64)>) {
        out.clear();
        match self {
            Self::Trapezoidal {
                root_chord_m,
                tip_chord_m,
                span_m,
                sweep_m,
            } => {
                if (0.0..*span_m).contains(&h) {
                    let f = h / span_m;
                    let lead = sweep_m * f;
                    out.push((lead, lead + root_chord_m + (tip_chord_m - root_chord_m) * f));
                }
            }
            Self::Elliptical {
                root_chord_m,
                span_m,
            } => {
                if (0.0..*span_m).contains(&h) {
                    let f = h / span_m;
                    let chord = root_chord_m * (1.0 - f * f).max(0.0).sqrt();
                    let lead = 0.5 * (root_chord_m - chord);
                    out.push((lead, lead + chord));
                }
            }
            Self::Freeform { points_m } => {
                let mut crossings: Vec<f64> = Vec::new();
                let n = points_m.len();
                for i in 0..n {
                    let p = points_m[i];
                    let q = points_m[(i + 1) % n];
                    let (lo, hi) = (p[1].min(q[1]), p[1].max(q[1]));
                    if p[1] != q[1] && h >= lo && h < hi {
                        crossings.push(p[0] + (h - p[1]) * (q[0] - p[0]) / (q[1] - p[1]));
                    }
                }
                crossings.sort_by(f64::total_cmp);
                let (pairs, _) = crossings.as_chunks::<2>();
                out.extend(pairs.iter().map(|&[a, b]| (a, b)));
            }
        }
    }

    /// Span stations where the chord intervals change form: 0, the span, and freeform vertices.
    fn breakpoints(&self) -> Vec<f64> {
        let mut points = vec![0.0, self.span_m()];
        if let Self::Freeform { points_m } = self {
            points.extend(points_m.iter().map(|p| p[1]));
        }
        points.sort_by(f64::total_cmp);
        points.dedup();
        points
    }

    /// Area and centroid of the planform.
    ///
    /// # Errors
    ///
    /// As [`FinPlanform::validate`], or [`DesignError::Numerics`] if an integral fails.
    pub fn geometry(&self) -> Result<PlanformGeometry, DesignError> {
        self.validate()?;
        let scale = self.root_chord_m() + self.span_m();
        let mut chords = Vec::new();
        let mut total = [0.0; 3];
        for pair in self.breakpoints().windows(2) {
            let piece = integrate(
                |h| {
                    self.chords(h * scale, &mut chords);
                    chords.iter().fold([0.0; 3], |acc, &(a, b)| {
                        let (a, b) = (a / scale, b / scale);
                        [
                            acc[0] + (b - a),
                            acc[1] + 0.5 * (b * b - a * a),
                            acc[2] + h * (b - a),
                        ]
                    })
                },
                pair[0] / scale,
                pair[1] / scale,
                PLATE,
            )?;
            for (sum, value) in total.iter_mut().zip(piece.value) {
                *sum += value;
            }
        }
        let area = total[0] * scale * scale;
        if area <= 0.0 {
            return Err(DesignError::Geometry("the fin has no area".to_owned()));
        }
        Ok(PlanformGeometry {
            area_m2: area,
            centroid_x_m: scale * total[1] / total[0],
            centroid_span_m: scale * total[2] / total[0],
        })
    }
}

/// Quadrature settings for plate integrands scaled to order one.
const PLATE: Tolerance = Tolerance {
    relative: 1e-12,
    absolute: 1e-16,
    max_intervals: 4000,
};

/// Checks a freeform outline: finite points, at least three, the first and last on the root,
/// none below it, positive area, and no two non-adjacent edges touching.
fn validate_outline(points: &[[f64; 2]]) -> Result<(), DesignError> {
    if points.len() < 3 {
        return Err(DesignError::Geometry(format!(
            "a freeform fin needs at least 3 points, got {}",
            points.len()
        )));
    }
    for p in points {
        for (what, value) in [("freeform fin x", p[0]), ("freeform fin span", p[1])] {
            if !value.is_finite() {
                return Err(DesignError::Domain { what, value });
            }
        }
        if p[1] < 0.0 {
            return Err(DesignError::Domain {
                what: "freeform fin span",
                value: p[1],
            });
        }
    }
    let (first, last) = (points[0], points[points.len() - 1]);
    if first[1] != 0.0 || last[1] != 0.0 || first[0] == last[0] {
        return Err(DesignError::Geometry(
            "a freeform fin must start and end at different points on the root (span 0)".to_owned(),
        ));
    }
    let n = points.len();
    let edge = |i: usize| (points[i], points[(i + 1) % n]);
    for i in 0..n {
        for j in (i + 1)..n {
            // Adjacent edges share a vertex; the closing edge is adjacent to the first.
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let (p, q) = edge(i);
            let (r, s) = edge(j);
            if segments_touch(p, q, r, s) {
                return Err(DesignError::Geometry(format!(
                    "the freeform fin outline crosses itself (edges {i} and {j})"
                )));
            }
        }
    }
    let twice_area: f64 = (0..n)
        .map(|i| {
            let (p, q) = edge(i);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    if twice_area == 0.0 {
        return Err(DesignError::Geometry("the fin has no area".to_owned()));
    }
    Ok(())
}

/// Whether the closed segments `pq` and `rs` share a point.
fn segments_touch(p: [f64; 2], q: [f64; 2], r: [f64; 2], s: [f64; 2]) -> bool {
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let within = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        c[0] >= a[0].min(b[0])
            && c[0] <= a[0].max(b[0])
            && c[1] >= a[1].min(b[1])
            && c[1] <= a[1].max(b[1])
    };
    let d1 = cross(r, s, p);
    let d2 = cross(r, s, q);
    let d3 = cross(p, q, r);
    let d4 = cross(p, q, s);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1 == 0.0 && within(r, s, p))
        || (d2 == 0.0 && within(r, s, q))
        || (d3 == 0.0 && within(p, q, r))
        || (d4 == 0.0 && within(p, q, s))
}

/// Volume integrals of a plate per unit density, about the fin-set origin with the fin at roll 0:
/// `[V, ∫r, ∫r², ∫x, ∫x², ∫rx, ∫τ²]` (each `dV`).
type PlateIntegrals = [f64; 7];

impl FinSet {
    /// Checks the set's dimensions and planform.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] or [`DesignError::Geometry`].
    pub fn validate(&self) -> Result<(), DesignError> {
        if self.count == 0 {
            return Err(DesignError::Domain {
                what: "fin count",
                value: 0.0,
            });
        }
        self.planform.validate()?;
        check_dimension("fin thickness", self.thickness_m, false)?;
        for (what, value) in [
            ("fin cant", self.cant_rad),
            ("fin base angle", self.base_angle_rad),
        ] {
            if !value.is_finite() {
                return Err(DesignError::Domain { what, value });
            }
        }
        if let Some(tab) = self.tab {
            check_dimension("fin tab height", tab.height_m, false)?;
            check_dimension("fin tab length", tab.length_m, false)?;
            if !tab.offset_m.is_finite() {
                return Err(DesignError::Domain {
                    what: "fin tab offset",
                    value: tab.offset_m,
                });
            }
        }
        Ok(())
    }

    /// The planform integrals of one fin, without its tab.
    fn fin_integrals(&self, body_radius_m: f64) -> Result<PlateIntegrals, DesignError> {
        let scale = self.planform.root_chord_m() + self.planform.span_m();
        let rb = body_radius_m / scale;
        let t = self.thickness_m / scale;
        let mut chords = Vec::new();
        let mut total = [0.0; 7];
        for pair in self.planform.breakpoints().windows(2) {
            let piece = integrate(
                |h| {
                    self.planform.chords(h * scale, &mut chords);
                    let mut m = [0.0; 4];
                    for &(a, b) in &chords {
                        let c = chord_moments(self.cross_section, a / scale, b / scale, t);
                        for k in 0..4 {
                            m[k] += c[k];
                        }
                    }
                    let r = rb + h;
                    [m[0], r * m[0], r * r * m[0], m[1], m[2], r * m[1], m[3]]
                },
                pair[0] / scale,
                pair[1] / scale,
                PLATE,
            )?;
            for (sum, value) in total.iter_mut().zip(piece.value) {
                *sum += value;
            }
        }
        // Undo the scaling: lengths to the powers 3, 4, 5, 4, 5, 5, 5.
        let powers = [3, 4, 5, 4, 5, 5, 5];
        Ok(std::array::from_fn(|k| total[k] * scale.powi(powers[k])))
    }

    /// The integrals of the tab slab `offset ≤ x ≤ offset + length`, `−height ≤ h ≤ 0`.
    fn tab_integrals(&self, body_radius_m: f64) -> PlateIntegrals {
        let Some(tab) = self.tab else {
            return [0.0; 7];
        };
        let t = self.thickness_m;
        let (x0, x1) = (tab.offset_m, tab.offset_m + tab.length_m);
        let (r0, r1) = (body_radius_m - tab.height_m, body_radius_m);
        let (h, l) = (tab.height_m, tab.length_m);
        let r_first = 0.5 * (r1 * r1 - r0 * r0);
        let x_first = 0.5 * (x1 * x1 - x0 * x0);
        [
            t * l * h,
            t * l * r_first,
            t * l * (r1.powi(3) - r0.powi(3)) / 3.0,
            t * h * x_first,
            t * h * (x1.powi(3) - x0.powi(3)) / 3.0,
            t * x_first * r_first,
            t.powi(3) / 12.0 * l * h,
        ]
    }

    /// Mass properties of one fin (and its tab) at roll angle 0 and no cant, in the fin-set
    /// frame, on a body of radius `body_radius_m`.
    ///
    /// # Errors
    ///
    /// As [`FinSet::validate`], plus [`DesignError::Domain`] for a negative body radius and
    /// [`DesignError::Numerics`] if an integral fails.
    pub fn single_fin(&self, body_radius_m: f64) -> Result<MassProperties, DesignError> {
        self.validate()?;
        check_dimension("fin body radius", body_radius_m, true)?;
        let density = self.material.bulk_kg_m3("fin set")?;
        let fin = self.fin_integrals(body_radius_m)?;
        let tab = self.tab_integrals(body_radius_m);
        let [v, r1, r2, x1, x2, rx, tau2] = std::array::from_fn(|k| fin[k] + tab[k]);
        if v <= 0.0 {
            return Err(DesignError::Geometry("the fin has no volume".to_owned()));
        }
        let mass = density * v;
        // Points sit at (r, τ, −x): I_xz = −∫ x_B z_B dm = ∫ r x dm.
        let about_origin = DMat3::from_cols(
            DVec3::new(tau2 + x2, 0.0, rx),
            DVec3::new(0.0, r2 + x2, 0.0),
            DVec3::new(rx, 0.0, r2 + tau2),
        ) * density;
        let cg = DVec3::new(r1 / v, 0.0, -x1 / v);
        let at_origin = MassProperties {
            mass_kg: mass,
            cg_m: cg,
            inertia_kg_m2: DMat3::ZERO,
        };
        // Move the tensor from the origin to the centre: I_cg = I_o − m(|c|²E − c cᵀ).
        let inertia = about_origin - at_origin.inertia_about(DVec3::ZERO);
        let fin = MassProperties {
            inertia_kg_m2: inertia,
            ..at_origin
        };
        if self.cant_rad == 0.0 {
            return Ok(fin);
        }
        let pivot = DVec3::new(body_radius_m, 0.0, -0.5 * self.planform.root_chord_m());
        Ok(fin
            .translated(-pivot)
            .rotated(DQuat::from_rotation_x(self.cant_rad))
            .translated(pivot))
    }

    /// Mass properties of the whole set in its frame, on a body of radius `body_radius_m`.
    ///
    /// # Errors
    ///
    /// As [`FinSet::single_fin`].
    pub fn mass_properties(&self, body_radius_m: f64) -> Result<MassProperties, DesignError> {
        let fin = self.single_fin(body_radius_m)?;
        let fins: Vec<MassProperties> = (0..self.count)
            .map(|k| {
                fin.rolled(self.base_angle_rad + 2.0 * PI * f64::from(k) / f64::from(self.count))
            })
            .collect();
        Ok(MassProperties::combine(&fins))
    }
}

/// Tube fins: open tubes parallel to the body, touching it, spaced evenly around it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TubeFinSet {
    /// Number of tubes, at least 1.
    pub count: u32,
    /// Length, m.
    pub length_m: f64,
    /// Outer radius, m.
    pub outer_radius_m: f64,
    /// Wall thickness, m.
    pub thickness_m: f64,
    /// Roll angle of the first tube's axis from `x_B` toward `y_B`, rad.
    #[serde(default)]
    pub base_angle_rad: f64,
    /// Material (bulk).
    pub material: Material,
}

impl TubeFinSet {
    /// Mass properties in the set's frame (origin on the body axis at the tubes' forward end), on
    /// a body of radius `body_radius_m`. Each tube is a hollow cylinder,
    /// `I_a = m(R² + r²)/2` and `I_t = m((R² + r²)/4 + L²/12)`, with its axis at `R_b + R`.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] for a bad dimension or count, [`DesignError::Geometry`] for a wall
    /// thicker than the radius, and material errors.
    pub fn mass_properties(&self, body_radius_m: f64) -> Result<MassProperties, DesignError> {
        if self.count == 0 {
            return Err(DesignError::Domain {
                what: "tube fin count",
                value: 0.0,
            });
        }
        check_dimension("body radius", body_radius_m, true)?;
        if !self.base_angle_rad.is_finite() {
            return Err(DesignError::Domain {
                what: "tube fin base angle",
                value: self.base_angle_rad,
            });
        }
        let density = self.material.bulk_kg_m3("tube fin set")?;
        let tube = crate::parts::hollow_cylinder(
            "tube fin",
            density,
            self.length_m,
            self.outer_radius_m,
            self.thickness_m,
        )?;
        let one = tube.translated(DVec3::new(body_radius_m + self.outer_radius_m, 0.0, 0.0));
        let tubes: Vec<MassProperties> = (0..self.count)
            .map(|k| {
                one.rolled(self.base_angle_rad + 2.0 * PI * f64::from(k) / f64::from(self.count))
            })
            .collect();
        Ok(MassProperties::combine(&tubes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hpr_core::quadrature::integrate_scalar;

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = if want == 0.0 {
            got.abs()
        } else {
            ((got - want) / want).abs()
        };
        assert!(err <= rel, "{what}: {got} vs {want} (relative {err:e})");
    }

    fn mat_close(a: DMat3, b: DMat3, rel: f64, what: &str) {
        let scale = b.to_cols_array().iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let diff = (a - b)
            .to_cols_array()
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(diff <= rel * scale, "{what}: {a:?}\nvs\n{b:?}");
    }

    fn ply() -> Material {
        Material::bulk("plywood", 630.0)
    }

    fn rectangle(chord: f64, span: f64) -> FinPlanform {
        FinPlanform::Trapezoidal {
            root_chord_m: chord,
            tip_chord_m: chord,
            span_m: span,
            sweep_m: 0.0,
        }
    }

    fn set(count: u32, planform: FinPlanform, thickness: f64) -> FinSet {
        FinSet {
            count,
            planform,
            thickness_m: thickness,
            cross_section: FinCrossSection::Square,
            tab: None,
            cant_rad: 0.0,
            base_angle_rad: 0.0,
            material: ply(),
        }
    }

    #[test]
    fn airfoil_constants_are_the_integrals() {
        // With ξ = u², dξ = 2u du, and the integrands are polynomials in u.
        let tol = Tolerance::default();
        let p = naca_polynomial;
        for (k, want) in AIRFOIL_PHI.iter().enumerate() {
            let power = i32::try_from(2 * k).unwrap();
            let got = 10.0
                * integrate_scalar(|u| u.powi(power) * p(u * u) * 2.0 * u, 0.0, 1.0, tol).unwrap();
            close(got, *want, 1e-14, "phi");
        }
        let psi = 1000.0 * integrate_scalar(|u| p(u * u).powi(3) * 2.0 * u, 0.0, 1.0, tol).unwrap();
        close(psi, AIRFOIL_PSI, 1e-14, "psi");
    }

    #[test]
    fn chord_moments_match_quadrature_of_each_section() {
        // The references ask for the tightest tolerance the quadrature allows (50 ε relative).
        let tol = Tolerance {
            relative: 0.0,
            absolute: 0.0,
            max_intervals: 4000,
        };
        let t: f64 = 0.004;
        // (a, b) wider than the thickness, and a sliver narrower than it (a circle of diameter c).
        for (a, b) in [(0.02, 0.17), (0.05, 0.053)] {
            let c: f64 = b - a;
            let rounded = |x: f64| {
                let mid = t.min(c);
                let r = 0.5 * mid;
                let u = (x - a).min(b - x);
                if u >= r {
                    mid
                } else {
                    2.0 * (r * r - (r - u).powi(2)).max(0.0).sqrt()
                }
            };
            let airfoil = |x: f64| 10.0 * t * naca_polynomial((x - a) / c);
            let square = |_x: f64| t;
            let sections: [(FinCrossSection, &dyn Fn(f64) -> f64); 3] = [
                (FinCrossSection::Square, &square),
                (FinCrossSection::Rounded, &rounded),
                (FinCrossSection::Airfoil, &airfoil),
            ];
            // Exact references: split where the edges meet the flat, and substitute x = a + v²
            // (and x = b − v²) at the ends, where the thickness grows like a square root.
            let edge = 0.5 * t.min(c);
            let reference = |f: &dyn Fn(f64) -> f64| -> f64 {
                let fore = integrate_scalar(|v| f(a + v * v) * 2.0 * v, 0.0, edge.sqrt(), tol);
                let aft = integrate_scalar(|v| f(b - v * v) * 2.0 * v, 0.0, edge.sqrt(), tol);
                let middle = integrate_scalar(f, a + edge, b - edge, tol);
                fore.unwrap() + aft.unwrap() + middle.unwrap()
            };
            for (section, thickness) in sections {
                let got = chord_moments(section, a, b, t);
                for (k, &moment) in got.iter().take(3).enumerate() {
                    let power = i32::try_from(k).unwrap();
                    let want = reference(&|x: f64| x.powi(power) * thickness(x));
                    close(
                        moment,
                        want,
                        1e-13,
                        &format!("{section:?} M{k} on [{a}, {b}]"),
                    );
                }
                let want = reference(&|x: f64| thickness(x).powi(3) / 12.0);
                close(got[3], want, 1e-13, &format!("{section:?} T on [{a}, {b}]"));
            }
        }
        // The rounded section of a wide chord loses (1 − π/4) t² of area.
        let m = chord_moments(FinCrossSection::Rounded, 0.0, 0.1, t);
        close(
            m[0],
            0.1 * t - (1.0 - PI / 4.0) * t * t,
            1e-14,
            "rounded area",
        );
    }

    #[test]
    fn planform_areas_and_centroids_are_the_closed_forms() {
        let (cr, ct, s, xt) = (0.15, 0.06, 0.1, 0.07);
        let trapezoid = FinPlanform::Trapezoidal {
            root_chord_m: cr,
            tip_chord_m: ct,
            span_m: s,
            sweep_m: xt,
        };
        let g = trapezoid.geometry().unwrap();
        close(g.area_m2, 0.5 * s * (cr + ct), 1e-13, "trapezoid area");
        close(
            g.centroid_x_m,
            (xt * (cr + 2.0 * ct) + cr * cr + cr * ct + ct * ct) / (3.0 * (cr + ct)),
            1e-13,
            "trapezoid centroid x",
        );
        close(
            g.centroid_span_m,
            s * (cr + 2.0 * ct) / (3.0 * (cr + ct)),
            1e-13,
            "trapezoid centroid h",
        );
        // The same outline as a freeform polygon.
        let freeform = FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [xt, s], [xt + ct, s], [cr, 0.0]],
        };
        let f = freeform.geometry().unwrap();
        close(f.area_m2, g.area_m2, 1e-13, "freeform area");
        close(f.centroid_x_m, g.centroid_x_m, 1e-13, "freeform centroid x");
        close(
            f.centroid_span_m,
            g.centroid_span_m,
            1e-13,
            "freeform centroid h",
        );
        // Half ellipse: area π c_r s / 4, centroid at mid-chord and 4s/3π out.
        let e = FinPlanform::Elliptical {
            root_chord_m: cr,
            span_m: s,
        }
        .geometry()
        .unwrap();
        close(e.area_m2, PI * cr * s / 4.0, 1e-12, "ellipse area");
        close(e.centroid_x_m, cr / 2.0, 1e-12, "ellipse centroid x");
        close(
            e.centroid_span_m,
            4.0 * s / (3.0 * PI),
            1e-12,
            "ellipse centroid h",
        );
    }

    #[test]
    fn a_rectangular_fin_set_matches_box_inertia_by_hand() {
        let (c, s, t, rb, rho) = (0.12, 0.08, 0.003, 0.04, 630.0);
        let m = rho * c * s * t;
        let d = rb + s / 2.0;
        // One fin: a box with s along x, t along y, c along z, centred at (d, 0, −c/2).
        let one = set(1, rectangle(c, s), t).single_fin(rb).unwrap();
        close(one.mass_kg, m, 1e-13, "mass");
        assert!((one.cg_m - DVec3::new(d, 0.0, -c / 2.0)).length() < 1e-15);
        let box_inertia = DMat3::from_diagonal(DVec3::new(
            m * (t * t + c * c) / 12.0,
            m * (s * s + c * c) / 12.0,
            m * (s * s + t * t) / 12.0,
        ));
        mat_close(one.inertia_kg_m2, box_inertia, 1e-12, "one fin");
        // Four fins: two along ±x and two along ±y.
        let four = set(4, rectangle(c, s), t).mass_properties(rb).unwrap();
        close(four.mass_kg, 4.0 * m, 1e-13, "four fins");
        assert!(four.cg_m.truncate().length() < 1e-15);
        let transverse =
            2.0 * m * (t * t + c * c) / 12.0 + 2.0 * (m * (s * s + c * c) / 12.0 + m * d * d);
        let axial = 4.0 * (m * (s * s + t * t) / 12.0 + m * d * d);
        mat_close(
            four.inertia_kg_m2,
            DMat3::from_diagonal(DVec3::new(transverse, transverse, axial)),
            1e-12,
            "four fins",
        );
        // Three fins are isotropic across the axis too; two are not.
        let three = set(3, rectangle(c, s), t)
            .mass_properties(rb)
            .unwrap()
            .inertia_kg_m2;
        close(
            three.x_axis.x,
            three.y_axis.y,
            1e-12,
            "three fins isotropic",
        );
        assert!(three.y_axis.x.abs() < 1e-12 * three.x_axis.x);
        let two = set(2, rectangle(c, s), t)
            .mass_properties(rb)
            .unwrap()
            .inertia_kg_m2;
        assert!((two.x_axis.x - two.y_axis.y).abs() > 1e-4 * two.y_axis.y);
    }

    #[test]
    fn cant_turns_the_fin_about_its_span_axis() {
        let (c, s, t, rb, rho) = (0.12, 0.08, 0.003, 0.04, 630.0);
        let m = rho * c * s * t;
        // Canted 90°, the chord lies along y: a box with s along x, c along y, t along z, whose
        // centre stays at the pivot's station because the pivot is the root mid-chord.
        let mut fins = set(1, rectangle(c, s), t);
        fins.cant_rad = PI / 2.0;
        let turned = fins.single_fin(rb).unwrap();
        assert!((turned.cg_m - DVec3::new(rb + s / 2.0, 0.0, -c / 2.0)).length() < 1e-15);
        let expected = DMat3::from_diagonal(DVec3::new(
            m * (c * c + t * t) / 12.0,
            m * (s * s + t * t) / 12.0,
            m * (s * s + c * c) / 12.0,
        ));
        mat_close(turned.inertia_kg_m2, expected, 1e-12, "canted 90°");
        // A small cant only couples x and... keeps mass and centre and the trace.
        fins.cant_rad = 0.05;
        let small = fins.single_fin(rb).unwrap();
        let flat = set(1, rectangle(c, s), t).single_fin(rb).unwrap();
        close(small.mass_kg, flat.mass_kg, 1e-15, "mass");
        let trace = |i: DMat3| i.x_axis.x + i.y_axis.y + i.z_axis.z;
        close(
            trace(small.inertia_kg_m2),
            trace(flat.inertia_kg_m2),
            1e-12,
            "trace",
        );
    }

    #[test]
    fn a_swept_fin_has_the_parallel_axis_product_of_inertia() {
        // A single trapezoidal fin: I_xz about the origin is ∫ r x dm, so about the centre it is
        // ∫ r x dm − m r̄ x̄ (with z = −x, I_xz = −∫ x_B z_B dm). Check against direct quadrature of
        // the planform.
        let (cr, ct, s, xt, t, rb, rho) = (0.15, 0.05, 0.1, 0.09, 0.004, 0.05, 1800.0);
        let planform = FinPlanform::Trapezoidal {
            root_chord_m: cr,
            tip_chord_m: ct,
            span_m: s,
            sweep_m: xt,
        };
        let mut fins = set(1, planform, t);
        fins.material = Material::bulk("G10", rho);
        let fin = fins.single_fin(rb).unwrap();
        let tol = Tolerance::default();
        let lead = |h: f64| xt * h / s;
        let chord = |h: f64| cr + (ct - cr) * h / s;
        let area = integrate_scalar(chord, 0.0, s, tol).unwrap();
        let mx = integrate_scalar(|h| chord(h) * (lead(h) + chord(h) / 2.0), 0.0, s, tol).unwrap();
        let mr = integrate_scalar(|h| chord(h) * (rb + h), 0.0, s, tol).unwrap();
        let mrx = integrate_scalar(
            |h| (rb + h) * chord(h) * (lead(h) + chord(h) / 2.0),
            0.0,
            s,
            tol,
        )
        .unwrap();
        let m = rho * t * area;
        close(fin.mass_kg, m, 1e-13, "mass");
        let product = rho * t * mrx - m * (mr / area) * (mx / area);
        close(fin.inertia_kg_m2.z_axis.x, product, 1e-11, "I_xz");
        close(fin.inertia_kg_m2.x_axis.z, product, 1e-11, "I_zx");
    }

    #[test]
    fn tube_fins_are_hollow_cylinders_around_the_body() {
        let tubes = TubeFinSet {
            count: 6,
            length_m: 0.1,
            outer_radius_m: 0.012,
            thickness_m: 0.001,
            base_angle_rad: 0.0,
            material: Material::bulk("cardboard", 790.0),
        };
        let rb = 0.02;
        let g = tubes.mass_properties(rb).unwrap();
        let (ro, ri) = (0.012, 0.011);
        let m1 = 790.0 * PI * (ro * ro - ri * ri) * 0.1;
        close(g.mass_kg, 6.0 * m1, 1e-13, "mass");
        let d = rb + ro;
        let axial = 6.0 * (0.5 * m1 * (ro * ro + ri * ri) + m1 * d * d);
        close(g.inertia_kg_m2.z_axis.z, axial, 1e-13, "axial");
        let transverse = 6.0 * m1 * ((ro * ro + ri * ri) / 4.0 + 0.01 / 12.0) + 3.0 * m1 * d * d;
        close(g.inertia_kg_m2.x_axis.x, transverse, 1e-12, "transverse");
        close(g.cg_m.z, -0.05, 1e-15, "centre");
    }

    #[test]
    fn bad_outlines_and_dimensions_are_rejected() {
        let bowtie = FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [0.1, 0.1], [0.0, 0.1], [0.1, 0.0]],
        };
        assert!(matches!(bowtie.validate(), Err(DesignError::Geometry(_))));
        let below = FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [0.05, -0.01], [0.1, 0.0]],
        };
        assert!(below.validate().is_err());
        let open = FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [0.05, 0.05], [0.1, 0.02]],
        };
        assert!(open.validate().is_err());
        assert!(
            set(0, rectangle(0.1, 0.1), 0.003)
                .mass_properties(0.03)
                .is_err()
        );
        assert!(
            set(3, rectangle(0.1, 0.1), 0.0)
                .mass_properties(0.03)
                .is_err()
        );
        let mut fabric = set(3, rectangle(0.1, 0.1), 0.003);
        fabric.material = Material::surface("ripstop", 0.04);
        assert!(matches!(
            fabric.mass_properties(0.03),
            Err(DesignError::MaterialKind { .. })
        ));
    }
}
