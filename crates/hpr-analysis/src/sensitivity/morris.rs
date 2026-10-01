//! Morris's screening: elementary effects along random one-at-a-time paths through a grid.
//!
//! **Guide:** [Sensitivity analysis][guide]'s *Morris screening* section.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/sensitivity.html#morris-screening
//!
//! # The method
//!
//! Each factor's range is mapped onto `[0, 1]` and cut into a grid of `p` levels, `0, 1/(p − 1),
//! …, 1`, with `p` even. A step is `Δ = p / (2 (p − 1))`, half the levels. The *elementary
//! effect* of factor `i` at a grid point `x` is
//!
//! `dᵢ(x) = (y(x₁, …, xᵢ + Δ, …, x_k) − y(x)) / Δ`
//!
//! for the points with `xᵢ ≤ 1 − Δ`. As `x` runs over the grid these form a finite
//! distribution, `Fᵢ`, of `p^(k−1) · p/2` effects. Its mean `μᵢ`, the mean of its absolute values
//! `μ*ᵢ`, and its standard deviation `σᵢ` say how much the factor matters: a large `μ*` is an
//! important factor, a large `σ` one that is nonlinear or acts with others, and a `μ*` near zero
//! one that can be left at its nominal value. Since `x` is on `[0, 1]`, `dᵢ` is in the output's
//! units per the factor's whole range: for `y = c x_i` in physical units, `dᵢ = c (high − low)`.
//!
//! A *path* (Morris's *trajectory*) is `k + 1` points: a random start, then each factor stepped
//! once by `±Δ`, in a random order. Each step gives one elementary effect, so `r` paths give `r`
//! effects of each factor from `r (k + 1)` runs. Morris's construction (his matrix `B*`, p. 164) draws
//! the start's coordinates from the levels `0, …, 1 − Δ`, a sign for each factor, and an order;
//! a factor whose sign is negative starts `Δ` higher and steps down. Each effect is then drawn
//! uniformly from `Fᵢ` (Morris, p. 164), so the means of the `r` effects estimate `μᵢ`, `μ*ᵢ` and
//! `σᵢ`, with a sampling error that falls as `1/√r` ([`ElementaryEffects`]).
//!
//! M. D. Morris, "Factorial sampling plans for preliminary computational experiments",
//! *Technometrics* 33(2), 161–174, 1991, <https://doi.org/10.2307/1269043>, defines the effects
//! (his eq. (1), p. 163), the grid, `Δ` and the paths. F. Campolongo, J. Cariboni and A.
//! Saltelli, "An effective screening design for sensitivity analysis of large models",
//! *Environmental Modelling & Software* 22, 1509–1518, 2007,
//! <https://doi.org/10.1016/j.envsoft.2006.10.004>, add `μ*`, which doesn't let effects of
//! opposite signs cancel (pp. 1511–1512), and find, by experiment rather than proof, that it
//! ranks factors as the total Sobol' index does (p. 1517). Not always. A step is about half the
//! range, so it misses a response that repeats over about half the range: on Ishigami's function
//! (on `[−π, π]`) a step of `x₂` is `pπ/(p − 1)`, near the period `π` of `sin² x₂`. At four levels
//! `μ*` puts `x₂` first (7.875 against 7.704 for `x₁`), the total index `x₁` (0.558 against
//! 0.442); at six or more, `μ*` puts `x₂` last (2.687 at six levels), though its first-order
//! index is the largest.
//!
//! [`Morris::population`] computes `Fᵢ`'s three moments exactly, by running the model at every
//! grid point: the numbers the paths estimate, for a model cheap enough to run `p^k` times.

use hpr_core::random::SeededRng;
use serde::{Deserialize, Serialize};

use super::{Factor, check_factors, check_outputs, check_size};
use crate::error::AnalysisError;

/// The most grid points [`Morris::population`] will run the model at.
pub const MAX_POPULATION_POINTS: usize = 1 << 24;

/// The most levels a grid may have: far more than a screening uses (the guide suggests 4).
pub const MAX_LEVELS: usize = 1 << 16;

/// A Morris screening: the factors, the grid's number of levels and the number of paths. It
/// serializes as its fields, and reads back through [`Morris::new`]'s checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "MorrisData")]
pub struct Morris {
    factors: Vec<Factor>,
    levels: usize,
    paths: usize,
}

/// The serialized form of a [`Morris`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MorrisData {
    factors: Vec<Factor>,
    levels: usize,
    paths: usize,
}

impl TryFrom<MorrisData> for Morris {
    type Error = AnalysisError;

    fn try_from(data: MorrisData) -> Result<Self, AnalysisError> {
        Self::new(data.factors, data.levels, data.paths)
    }
}

/// One path: where it starts on the grid, and which way and in which order its factors step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Path {
    /// Each factor's level at the start, `0` to `p − 1`.
    pub start: Vec<usize>,
    /// The factors in the order they step.
    pub order: Vec<usize>,
    /// Each factor's direction: up `Δ` (`true`) or down.
    pub up: Vec<bool>,
}

/// The points of a Morris screening, path after path, each path's `k + 1` points in order. It is
/// not serialized: [`Morris::design`] rebuilds it, bit for bit, from the screening and its seed.
#[derive(Debug, Clone, PartialEq)]
pub struct MorrisDesign {
    factors: Vec<Factor>,
    levels: usize,
    paths: Vec<Path>,
}

/// A factor's elementary effects: from a screening's paths ([`MorrisDesign::analyse`]), or all
/// of them on the grid ([`Morris::population`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ElementaryEffects {
    /// The factor's name.
    pub name: String,
    /// How many effects: one per path, or every one on the grid.
    pub count: usize,
    /// Their mean, `μ`, in the output's units per the factor's whole range.
    pub mean: f64,
    /// The mean of their absolute values, `μ*`, in the same units.
    pub mean_absolute: f64,
    /// Their standard deviation, `σ`: from paths with `count − 1` in the denominator, of the
    /// whole grid with `count`.
    pub standard_deviation: f64,
    /// The standard error of `μ*` from paths: the standard deviation of the absolute effects
    /// over `√count`. Zero for the whole grid, which is exact.
    pub mean_absolute_standard_error: f64,
}

/// What a Morris screening found: each factor's effects, in the factors' order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Screening {
    /// The grid's number of levels, `p`.
    pub levels: usize,
    /// The step, `Δ`, as a fraction of each factor's range.
    pub step: f64,
    /// Each factor's effects.
    pub effects: Vec<ElementaryEffects>,
}

impl Morris {
    /// A screening of `factors` on a grid of `levels` levels, with `paths` paths.
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::TooFew`] with no factors, fewer than 2 levels, or fewer than 2 paths
    ///   (a standard deviation needs two).
    /// - [`AnalysisError::DuplicateFactor`] for two factors with one name.
    /// - [`AnalysisError::Unsupported`] for an odd number of levels: Morris's step `Δ` is a whole
    ///   number of levels only for an even one.
    /// - [`AnalysisError::Count`] for more than [`MAX_LEVELS`] levels, or a design of more than
    ///   [`MAX_DESIGN_POINTS`](super::MAX_DESIGN_POINTS) points or
    ///   [`MAX_DESIGN_VALUES`](super::MAX_DESIGN_VALUES) coordinates.
    pub fn new(factors: Vec<Factor>, levels: usize, paths: usize) -> Result<Self, AnalysisError> {
        check_factors(&factors)?;
        if levels < 2 {
            return Err(AnalysisError::TooFew {
                what: "grid levels",
                count: levels,
                minimum: 2,
            });
        }
        if levels > MAX_LEVELS {
            return Err(AnalysisError::Count {
                what: "grid levels",
                count: levels,
                limit: MAX_LEVELS,
            });
        }
        if !levels.is_multiple_of(2) {
            return Err(AnalysisError::Unsupported(format!(
                "{levels} grid levels: Morris's step is a whole number of levels only for an even number"
            )));
        }
        if paths < 2 {
            return Err(AnalysisError::TooFew {
                what: "Morris paths",
                count: paths,
                minimum: 2,
            });
        }
        let k = factors.len();
        check_size("Morris points", paths, k.saturating_add(1), k)?;
        Ok(Self {
            factors,
            levels,
            paths,
        })
    }

    /// The factors.
    pub fn factors(&self) -> &[Factor] {
        &self.factors
    }

    /// The grid's number of levels, `p`.
    pub fn levels(&self) -> usize {
        self.levels
    }

    /// The number of paths, `r`.
    pub fn paths(&self) -> usize {
        self.paths
    }

    /// The step `Δ = p / (2 (p − 1))`, as a fraction of each factor's range.
    pub fn step(&self) -> f64 {
        step(self.levels)
    }

    /// The paths of a screening seeded with `seed`. Path `j` draws from its own stream,
    /// [`SeededRng::for_stream`]`(seed, &[j])`: its start's levels, then a direction for each
    /// factor, then the order (a Fisher–Yates shuffle), so it is the same however many paths are
    /// drawn.
    pub fn design(&self, seed: u64) -> MorrisDesign {
        let k = self.factors.len();
        let half = self.levels / 2;
        let paths = (0..self.paths)
            .map(|j| {
                // Cast: a path's index is far below 2⁶⁴.
                let mut rng = SeededRng::for_stream(seed, &[j as u64]);
                let start: Vec<usize> = (0..k).map(|_| pick(&mut rng, half)).collect();
                let up: Vec<bool> = (0..k).map(|_| rng.uniform() < 0.5).collect();
                let mut order: Vec<usize> = (0..k).collect();
                for i in (1..k).rev() {
                    order.swap(i, pick(&mut rng, i + 1));
                }
                // A factor that steps down starts half the levels higher.
                let start = start
                    .iter()
                    .zip(&up)
                    .map(|(&level, &up)| if up { level } else { level + half })
                    .collect();
                Path { start, order, up }
            })
            .collect();
        MorrisDesign {
            factors: self.factors.clone(),
            levels: self.levels,
            paths,
        }
    }

    /// Draws the design seeded with `seed`, runs `model` at each of its points in order, and
    /// analyses the outputs ([`MorrisDesign::analyse`]).
    ///
    /// # Errors
    ///
    /// As [`MorrisDesign::analyse`].
    pub fn screen(
        &self,
        seed: u64,
        mut model: impl FnMut(&[f64]) -> f64,
    ) -> Result<Screening, AnalysisError> {
        let design = self.design(seed);
        let outputs: Vec<f64> = design.points().iter().map(|x| model(x)).collect();
        design.analyse(&outputs)
    }

    /// Each factor's elementary effects over the whole grid: `Fᵢ`'s mean, mean absolute value and
    /// standard deviation, exactly, from `model` run once at each of the grid's `p^k` points.
    /// These are what [`Morris::screen`] estimates.
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::Count`] for a grid of more than [`MAX_POPULATION_POINTS`] points, its
    ///   count `p^k` saturating at `usize::MAX`.
    /// - [`AnalysisError::Output`] for an output that isn't finite, at its grid point's index
    ///   (factor 0's level the fastest-changing digit).
    pub fn population(
        &self,
        mut model: impl FnMut(&[f64]) -> f64,
    ) -> Result<Vec<ElementaryEffects>, AnalysisError> {
        let k = self.factors.len();
        let p = self.levels;
        let total = (0..k).fold(1_usize, |n, _| n.saturating_mul(p));
        if total > MAX_POPULATION_POINTS {
            return Err(AnalysisError::Count {
                what: "Morris grid points",
                count: total,
                limit: MAX_POPULATION_POINTS,
            });
        }
        let mut levels = vec![0_usize; k];
        let mut point = vec![0.0; k];
        let mut outputs = Vec::with_capacity(total);
        for index in 0..total {
            let mut rest = index;
            for (level, (x, factor)) in levels.iter_mut().zip(point.iter_mut().zip(&self.factors)) {
                *level = rest % p;
                rest /= p;
                *x = factor.at(unit(*level, p));
            }
            let y = model(&point);
            if !y.is_finite() {
                return Err(AnalysisError::Output { index, value: y });
            }
            outputs.push(y);
        }
        let delta = self.step();
        let half = p / 2;
        let mut stride = 1;
        let mut effects = Vec::with_capacity(k);
        for factor in &self.factors {
            let mut ds = Vec::with_capacity(total / 2);
            for (index, &lower) in outputs.iter().enumerate() {
                if (index / stride) % p < half {
                    ds.push((outputs[index + half * stride] - lower) / delta);
                }
            }
            effects.push(moments(factor.name(), &ds, false));
            stride *= p;
        }
        Ok(effects)
    }
}

impl MorrisDesign {
    /// The paths.
    pub fn paths(&self) -> &[Path] {
        &self.paths
    }

    /// The number of points, `r (k + 1)`.
    pub fn len(&self) -> usize {
        self.paths.len() * (self.factors.len() + 1)
    }

    /// Whether there are no points; never, as a screening has at least two paths.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The points, in physical units, path after path: each path's start, then the point after
    /// each step in its order.
    pub fn points(&self) -> Vec<Vec<f64>> {
        let half = self.levels / 2;
        let p = self.levels;
        let mut points = Vec::with_capacity(self.len());
        for path in &self.paths {
            let mut levels = path.start.clone();
            points.push(self.at(&levels));
            for &i in &path.order {
                levels[i] = if path.up[i] {
                    levels[i] + half
                } else {
                    levels[i] - half
                };
                debug_assert!(levels[i] < p, "a path stays on the grid");
                points.push(self.at(&levels));
            }
        }
        points
    }

    /// The point at grid levels `levels`, in physical units.
    fn at(&self, levels: &[usize]) -> Vec<f64> {
        levels
            .iter()
            .zip(&self.factors)
            .map(|(&level, factor)| factor.at(unit(level, self.levels)))
            .collect()
    }

    /// Each factor's elementary effects from `outputs`, the model's output at each of
    /// [`MorrisDesign::points`] in order. A step up of factor `i` from point `x` to `x′` gives
    /// `(y(x′) − y(x))/Δ`, a step down `(y(x) − y(x′))/Δ`, so either is the effect at the lower
    /// point.
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::Length`] unless there is one output per point.
    /// - [`AnalysisError::Output`] for an output that isn't finite.
    pub fn analyse(&self, outputs: &[f64]) -> Result<Screening, AnalysisError> {
        check_outputs(outputs, self.len())?;
        let k = self.factors.len();
        let delta = step(self.levels);
        let mut effects = (0..k)
            .map(|_| Vec::with_capacity(self.paths.len()))
            .collect::<Vec<_>>();
        for (path, ys) in self.paths.iter().zip(outputs.chunks_exact(k + 1)) {
            for (s, &i) in path.order.iter().enumerate() {
                let change = ys[s + 1] - ys[s];
                effects[i].push(if path.up[i] { change } else { -change } / delta);
            }
        }
        Ok(Screening {
            levels: self.levels,
            step: delta,
            effects: self
                .factors
                .iter()
                .zip(&effects)
                .map(|(factor, ds)| moments(factor.name(), ds, true))
                .collect(),
        })
    }
}

/// `Δ = p / (2 (p − 1))`.
fn step(levels: usize) -> f64 {
    // Cast: a number of levels is far below 2⁵³.
    (levels / 2) as f64 / (levels - 1) as f64
}

/// Grid level `level` of `levels` on `[0, 1]`.
fn unit(level: usize, levels: usize) -> f64 {
    // Cast: a number of levels is far below 2⁵³.
    level as f64 / (levels - 1) as f64
}

/// A whole number in `0..n`, from one uniform deviate.
fn pick(rng: &mut SeededRng, n: usize) -> usize {
    // Cast: `n` is far below 2⁵³, and `u < 1` keeps the product below `n`; `min` states it.
    ((rng.uniform() * n as f64) as usize).min(n - 1)
}

/// The moments of a factor's effects `ds`: from paths (`sample`, with `n − 1` and a standard
/// error), or the whole grid (with `n`, exact).
fn moments(name: &str, ds: &[f64], sample: bool) -> ElementaryEffects {
    // Cast: a count of effects is far below 2⁵³.
    let n = ds.len() as f64;
    let mean = ds.iter().sum::<f64>() / n;
    let mean_absolute = ds.iter().map(|d| d.abs()).sum::<f64>() / n;
    let squares = |centre: f64, abs: bool| {
        ds.iter()
            .map(|&d| {
                let e = if abs { d.abs() } else { d } - centre;
                e * e
            })
            .sum::<f64>()
    };
    let (standard_deviation, mean_absolute_standard_error) = if sample {
        (
            (squares(mean, false) / (n - 1.0)).sqrt(),
            (squares(mean_absolute, true) / (n - 1.0) / n).sqrt(),
        )
    } else {
        ((squares(mean, false) / n).sqrt(), 0.0)
    };
    ElementaryEffects {
        name: name.to_owned(),
        count: ds.len(),
        mean,
        mean_absolute,
        standard_deviation,
        mean_absolute_standard_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_factors(k: usize) -> Vec<Factor> {
        (0..k)
            .map(|i| Factor::new(format!("x{i}"), 0.0, 1.0).unwrap())
            .collect()
    }

    #[test]
    fn the_step_is_half_the_levels() {
        let m = Morris::new(unit_factors(2), 4, 2).unwrap();
        assert_eq!(m.step(), 2.0 / 3.0);
        assert_eq!(Morris::new(unit_factors(2), 2, 2).unwrap().step(), 1.0);
        assert_eq!(
            Morris::new(unit_factors(2), 8, 2).unwrap().step(),
            4.0 / 7.0
        );
    }

    #[test]
    fn a_screening_refuses_what_it_cant_lay_out() {
        assert!(matches!(
            Morris::new(Vec::new(), 4, 10),
            Err(AnalysisError::TooFew {
                what: "factors",
                ..
            })
        ));
        assert!(matches!(
            Morris::new(unit_factors(2), 0, 10),
            Err(AnalysisError::TooFew {
                what: "grid levels",
                count: 0,
                ..
            })
        ));
        match Morris::new(unit_factors(2), 5, 10) {
            Err(AnalysisError::Unsupported(why)) => assert!(why.starts_with("5 grid levels")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            Morris::new(unit_factors(2), MAX_LEVELS + 2, 10),
            Err(AnalysisError::Count {
                what: "grid levels",
                limit: MAX_LEVELS,
                ..
            })
        ));
        assert!(Morris::new(unit_factors(2), MAX_LEVELS, 10).is_ok());
        assert!(matches!(
            Morris::new(unit_factors(2), 4, 1),
            Err(AnalysisError::TooFew {
                what: "Morris paths",
                count: 1,
                ..
            })
        ));
        // Points past the limit: refused when laid out, not when allocated.
        assert!(matches!(
            Morris::new(unit_factors(1), 4, 1 << 62),
            Err(AnalysisError::Count {
                what: "Morris points",
                limit: crate::sensitivity::MAX_DESIGN_POINTS,
                ..
            })
        ));
        assert!(Morris::new(unit_factors(1), 4, 1 << 19).is_ok());
        assert!(matches!(
            Morris::new(unit_factors(2), 4, usize::MAX / 2),
            Err(AnalysisError::Count {
                what: "Morris points",
                count: usize::MAX,
                ..
            })
        ));
    }

    #[test]
    fn every_path_steps_each_factor_once_by_delta_and_stays_on_the_grid() {
        let m = Morris::new(unit_factors(5), 6, 40).unwrap();
        let design = m.design(7);
        let points = design.points();
        assert_eq!(points.len(), 40 * 6);
        for (path, chunk) in design.paths().iter().zip(points.as_chunks::<6>().0) {
            let mut order = path.order.clone();
            order.sort_unstable();
            assert_eq!(order, (0..5).collect::<Vec<_>>());
            for (s, &i) in path.order.iter().enumerate() {
                for (j, (after, before)) in chunk[s + 1].iter().zip(&chunk[s]).enumerate() {
                    let moved = after - before;
                    if j == i {
                        let expected = if path.up[i] { 0.6 } else { -0.6 };
                        assert!((moved - expected).abs() < 1e-15, "{moved}");
                    } else {
                        assert_eq!(moved, 0.0);
                    }
                }
            }
            for x in chunk.iter().flatten() {
                assert!((0.0..=1.0).contains(x));
                let level = x * 5.0;
                assert!((level - level.round()).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn path_j_is_the_same_however_many_are_drawn() {
        let short = Morris::new(unit_factors(4), 4, 3).unwrap().design(11);
        let long = Morris::new(unit_factors(4), 4, 30).unwrap().design(11);
        assert_eq!(short.paths(), &long.paths()[..3]);
        assert_ne!(long.paths()[3], long.paths()[4]);
        let other = Morris::new(unit_factors(4), 4, 3).unwrap().design(12);
        assert_ne!(short.paths(), other.paths());
    }

    #[test]
    fn a_linear_model_has_its_slopes_as_every_effect() {
        // y = 3 a − 2 b + 0 c, with b's range 4 wide: its effects are its slope times its range.
        let factors = vec![
            Factor::new("a", 0.0, 1.0).unwrap(),
            Factor::new("b", -1.0, 3.0).unwrap(),
            Factor::new("c", 5.0, 6.0).unwrap(),
        ];
        let screening = Morris::new(factors, 4, 12)
            .unwrap()
            .screen(3, |x| 3.0 * x[0] - 2.0 * x[1])
            .unwrap();
        let expected = [3.0, -8.0, 0.0];
        for (e, want) in screening.effects.iter().zip(expected) {
            assert_eq!(e.count, 12);
            assert!((e.mean - want).abs() < 1e-12, "{e:?}");
            assert!((e.mean_absolute - want.abs()).abs() < 1e-12, "{e:?}");
            assert!(e.standard_deviation < 1e-12, "{e:?}");
            assert!(e.mean_absolute_standard_error < 1e-12, "{e:?}");
        }
    }

    #[test]
    fn a_step_down_gives_the_effect_at_the_lower_point() {
        // y = x² on [0, 1], two levels: Δ = 1 and the one effect is y(1) − y(0) = 1, both ways.
        let m = Morris::new(unit_factors(1), 2, 6).unwrap();
        let design = m.design(1);
        assert!(design.paths().iter().any(|p| p.up[0]));
        assert!(design.paths().iter().any(|p| !p.up[0]));
        let outputs: Vec<f64> = design.points().iter().map(|x| x[0] * x[0]).collect();
        let e = &design.analyse(&outputs).unwrap().effects[0];
        assert_eq!(
            (e.mean, e.mean_absolute, e.standard_deviation),
            (1.0, 1.0, 0.0)
        );
    }

    #[test]
    fn the_population_of_a_product_is_its_closed_form() {
        // y = x₀ x₁ on [0, 1]², p = 4, Δ = 2/3: d₀ = x₁ at x₁'s four levels, each twice.
        let m = Morris::new(unit_factors(2), 4, 2).unwrap();
        let population = m.population(|x| x[0] * x[1]).unwrap();
        let levels = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
        let mean = levels.iter().sum::<f64>() / 4.0;
        let variance = levels.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / 4.0;
        for e in &population {
            assert_eq!(e.count, 8);
            assert!((e.mean - mean).abs() < 1e-15);
            assert!((e.mean_absolute - mean).abs() < 1e-15);
            assert!((e.standard_deviation - variance.sqrt()).abs() < 1e-15);
            assert_eq!(e.mean_absolute_standard_error, 0.0);
        }
    }

    #[test]
    fn the_population_refuses_a_grid_too_large_and_names_a_bad_point() {
        let m = Morris::new(unit_factors(13), 4, 2).unwrap();
        assert!(matches!(
            m.population(|_| 0.0),
            Err(AnalysisError::Count {
                what: "Morris grid points",
                count: 67_108_864,
                limit: MAX_POPULATION_POINTS,
            })
        ));
        let m = Morris::new(unit_factors(2), 4, 2).unwrap();
        // Index 6 is x₀ at level 2 and x₁ at level 1.
        match m.population(|x| {
            if x == [2.0 / 3.0, 1.0 / 3.0] {
                f64::NAN
            } else {
                0.0
            }
        }) {
            Err(AnalysisError::Output { index, .. }) => assert_eq!(index, 6),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_screening_reads_back_through_its_checks() {
        let m = Morris::new(unit_factors(2), 4, 10).unwrap();
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Morris>(&json).unwrap(), m);
        let odd = json.replace("\"levels\":4", "\"levels\":3");
        let refused = serde_json::from_str::<Morris>(&odd)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("3 grid levels"), "{refused}");
    }
}
