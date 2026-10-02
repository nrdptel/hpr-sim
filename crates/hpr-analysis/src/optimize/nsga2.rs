//! NSGA-II, the non-dominated sorting genetic algorithm II: a search for the designs that trade
//! two or more goals off against each other, the *Pareto front*.
//!
//! A design *dominates* another when it is no worse in any goal and better in at least one. The
//! designs no other design dominates are the Pareto front: along it, one goal can only be
//! improved by giving up another. NSGA-II keeps a population of designs, breeds a new generation
//! from them, and keeps the best half of parents and children together. They are ranked first by
//! how many fronts deep they lie, then, within a front, by how far they are from their
//! neighbours, so the front it finds spreads along the true one instead of bunching at one place.
//!
//! The method is K. Deb, A. Pratap, S. Agarwal and T. Meyarivan, "A fast and elitist
//! multiobjective genetic algorithm: NSGA-II", *IEEE Transactions on Evolutionary Computation*
//! 6(2), 182–197 (2002), <https://doi.org/10.1109/4235.996017>:
//!
//! - **Fast non-dominated sorting** (§III-A, p. 184): each design's count of designs dominating
//!   it, and the set it dominates; the designs with a count of zero are the first front, and
//!   removing them gives the next. Rank 0 here is the paper's front 1.
//! - **Crowding distance** (§III-B, p. 185): in each front and for each goal, the designs sorted
//!   by that goal; the two ends get an infinite distance, and every other design adds the gap
//!   between its two neighbours' values divided by the goal's range in the front,
//!   `(f_m(i+1) − f_m(i−1)) / (f_m^max − f_m^min)`.
//! - **Crowded comparison** (p. 185): a lower rank wins; in one rank, the larger distance wins.
//! - **The main loop** (§III-C, p. 186): parents and children together, `2N` designs, are
//!   sorted into fronts; whole fronts fill the next `N` in rank order, and the front that doesn't
//!   fit is cut by crowding distance, largest first.
//! - **Constraints** (§VI, p. 192): design `i` *constrained-dominates* `j` if `i` keeps every
//!   constraint and `j` doesn't, if neither does and `i`'s overall violation is smaller, or if both
//!   do and `i` dominates `j`.
//!
//! Children come from parents chosen by binary tournaments with the crowded comparison, two at a
//! time, by the paper's real-coded operators (§IV, p. 187):
//!
//! - **Simulated binary crossover** (SBX; K. Deb and R. B. Agrawal, "Simulated binary crossover
//!   for continuous search space", *Complex Systems* 9(2), 115–148 (1995)): a child's spread
//!   factor `β = |c₂ − c₁| / |x₂ − x₁|` has density `½(η_c + 1) β^η_c` for `β ≤ 1` and
//!   `½(η_c + 1) / β^(η_c+2)` above (eqs. 19–20, pp. 125–126). Each variable of a crossing pair is
//!   crossed with probability ½ (K. Deb and H.-G. Beyer, "Self-adaptive genetic algorithms with
//!   simulated binary crossover", report CI-61/99, Univ. Dortmund (1999), p. 8), and the two
//!   children's values swap with probability ½.
//! - **Polynomial mutation** (K. Deb and M. Goyal, "A combined genetic adaptive search (GeneAS)
//!   for engineering design", *Computer Science and Informatics* 26(4), 30–45 (1996)).
//!
//! Both are cut at the variable's bounds, so no child is placed outside and none piles up on
//! the bound. SBX drops the density beyond the bound and scales the rest up to a whole, as Deb
//! and Agrawal propose (p. 143). Mutation cuts each side of the value at its own bound and keeps
//! half the probability on each side. The equations of both cuts, given with each function
//! below, are those of Deb's NSGA-II code as pymoo 0.6.2 (Apache-2.0) prints them, in its
//! `operators/crossover/sbx.py` and `operators/mutation/pm.py`; no paper we could reach prints
//! them, so each is derived again in its function's comment.
//!
//! The defaults are the paper's real-coded settings (p. 187): crossover probability 0.9,
//! mutation probability `1/n` per variable, distribution indices `η_c = η_m = 20`; the
//! population of 100 and 250 generations are its runs on the ZDT problems (p. 187).
//!
//! # Variables
//!
//! NSGA-II draws its first population uniformly between each variable's bounds, so every
//! variable needs two finite bounds ([`Variable::within`]); its start and step are not used.
//! Integer variables are not taken yet.
//!
//! # Reproducibility
//!
//! A generation's random numbers come from one stream keyed by the seed and the generation's
//! number ([`SeededRng::for_stream`]), and are drawn before its children are evaluated, so a run
//! is bit for bit the same every time on one platform, however its children are evaluated.
//! Ties in a sort keep the designs' order, parents before children.

use serde::{Deserialize, Serialize};

use hpr_core::random::SeededRng;

use super::{Variable, check_variables};
use crate::error::AnalysisError;

/// The largest population a run takes, 2¹². The sort of parents and children together keeps, for
/// each design, the list of designs it dominates: up to about `(2N)²/2` indices, some 270 MB on a
/// 64-bit machine at this size (more while the lists grow), and four times as much for each
/// doubling.
pub const MAX_POPULATION: usize = 1 << 12;

/// The largest bound a variable may have, in size: `f64::MAX/4`, so that crossover's sums and
/// spreads stay finite.
pub const MAX_BOUND: f64 = f64::MAX / 4.0;

/// The most goals a run takes.
pub const MAX_OBJECTIVES: usize = 16;

/// The optimizer's settings: the variables, the number of goals, the population, the number of
/// generations, and the crossover's and mutation's parameters. It serializes as its fields, and
/// reads back through the same checks as [`Nsga2::new`] and its `with_` methods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Nsga2Data")]
pub struct Nsga2 {
    variables: Vec<Variable>,
    objectives: usize,
    population: usize,
    generations: usize,
    crossover_probability: f64,
    crossover_index: f64,
    mutation_probability: f64,
    mutation_index: f64,
}

/// The serialized form of an [`Nsga2`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nsga2Data {
    variables: Vec<Variable>,
    objectives: usize,
    population: usize,
    generations: usize,
    crossover_probability: f64,
    crossover_index: f64,
    mutation_probability: f64,
    mutation_index: f64,
}

impl TryFrom<Nsga2Data> for Nsga2 {
    type Error = AnalysisError;

    fn try_from(data: Nsga2Data) -> Result<Self, AnalysisError> {
        Nsga2::new(data.variables, data.objectives)?
            .with_population(data.population)?
            .with_generations(data.generations)?
            .with_crossover(data.crossover_probability, data.crossover_index)?
            .with_mutation(data.mutation_probability, data.mutation_index)
    }
}

/// What a model gives for one design: its goals, each to be made as small as it can be, and by
/// how much it breaks the constraints, zero if it keeps them all.
///
/// A design the model can't evaluate (a flight that fails) is [`Goals::failed`]: its violation
/// is `+∞`, so every design that could be evaluated dominates it. Infinite values serialize as
/// none, a JSON `null`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Goals {
    /// The goals' values, in the order the model gives them.
    #[serde(with = "infinities_as_none")]
    pub objectives: Vec<f64>,
    /// The total violation, `Σ max(0, gⱼ)` over constraints written `gⱼ ≤ 0`: zero if the design
    /// keeps them all, `+∞` if it failed.
    #[serde(with = "super::cmaes::infinity_as_none")]
    pub violation: f64,
}

impl Goals {
    /// Goals with no constraints to break.
    pub fn feasible(objectives: Vec<f64>) -> Self {
        Self {
            objectives,
            violation: 0.0,
        }
    }

    /// Goals under constraints `gⱼ(x) ≤ 0`, given as the numbers `gⱼ`: the violation is
    /// `Σ max(0, gⱼ)`, as for [`Evaluation::constrained`](super::Evaluation::constrained), whose
    /// advice on scaling the constraints holds here too.
    pub fn constrained(objectives: Vec<f64>, constraints: &[f64]) -> Self {
        Self {
            objectives,
            violation: super::Evaluation::constrained(0.0, constraints).violation,
        }
    }

    /// A design the model can't evaluate: no goals, and a violation of `+∞`, behind every other.
    pub fn failed() -> Self {
        Self {
            objectives: Vec::new(),
            violation: f64::INFINITY,
        }
    }

    /// Whether the design keeps every constraint.
    pub fn is_feasible(&self) -> bool {
        self.violation == 0.0
    }
}

/// One design of a population: where it is, what the model gave for it, and where it ranks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Member {
    /// The design, one value per variable, in the variables' order.
    pub point: Vec<f64>,
    /// Its goals' values, `+∞` each for a failed design (serialized as none, a JSON `null`).
    #[serde(with = "infinities_as_none")]
    pub objectives: Vec<f64>,
    /// Its constraint violation: zero if it keeps every constraint, `+∞` if it failed
    /// (serialized as none).
    #[serde(with = "super::cmaes::infinity_as_none")]
    pub violation: f64,
    /// Its front, counted from 0: rank 0 is the designs no other in the sort dominated.
    pub rank: usize,
    /// Its crowding distance in its front, `+∞` at a front's ends (serialized as none).
    #[serde(with = "super::cmaes::infinity_as_none")]
    pub crowding: f64,
}

/// What a run found: its last population's first front, and the whole population.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Front {
    /// The designs of rank 0 in the last population, in the population's order: the run's
    /// estimate of the Pareto front. If no design kept every constraint, these are the ones
    /// that broke them least: check [`Front::is_feasible`].
    pub members: Vec<Member>,
    /// The last population, every rank, ranked as the run left it.
    pub population: Vec<Member>,
    /// How many evaluations the run made: the population times the generations.
    pub evaluations: usize,
    /// How many generations it ran, the first, random one included.
    pub generations: usize,
}

impl Front {
    /// Whether every design of the front keeps every constraint.
    pub fn is_feasible(&self) -> bool {
        self.members.iter().all(|m| m.violation == 0.0)
    }
}

/// The serialized form of goals that are finite or `+∞`: options, none for `+∞`.
mod infinities_as_none {
    use serde::ser::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(x: &[f64], serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(bad) = x.iter().find(|x| x.is_nan() || **x == f64::NEG_INFINITY) {
            return Err(S::Error::custom(format!("{bad} is neither finite nor +∞")));
        }
        let values: Vec<Option<f64>> = x
            .iter()
            .map(|&x| (x != f64::INFINITY).then_some(x))
            .collect();
        values.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<f64>, D::Error> {
        Ok(Vec::<Option<f64>>::deserialize(deserializer)?
            .into_iter()
            .map(|x| x.unwrap_or(f64::INFINITY))
            .collect())
    }
}

/// A distribution index: finite and not negative.
fn check_index(what: &'static str, index: f64) -> Result<f64, AnalysisError> {
    if index.is_finite() && index >= 0.0 {
        Ok(index)
    } else {
        Err(AnalysisError::Domain { what, value: index })
    }
}

/// A probability: within `[0, 1]`.
fn check_probability(what: &'static str, p: f64) -> Result<f64, AnalysisError> {
    if (0.0..=1.0).contains(&p) {
        Ok(p)
    } else {
        Err(AnalysisError::Domain { what, value: p })
    }
}

impl Nsga2 {
    /// The optimizer over `variables` for `objectives` goals, with the paper's settings: a
    /// population of 100, 250 generations, crossover probability 0.9 and `η_c = 20`, mutation
    /// probability `1/n` and `η_m = 20`.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] or [`AnalysisError::Count`] for no variables or more than
    /// [`MAX_VARIABLES`](super::MAX_VARIABLES), or no goals or more than [`MAX_OBJECTIVES`];
    /// [`AnalysisError::DuplicateVariable`] for two variables of one name;
    /// [`AnalysisError::Domain`] for a variable without two finite bounds, or with one beyond
    /// [`MAX_BOUND`]; and
    /// [`AnalysisError::Unsupported`] for an integer variable.
    pub fn new(variables: Vec<Variable>, objectives: usize) -> Result<Self, AnalysisError> {
        check_variables(&variables)?;
        if objectives == 0 {
            return Err(AnalysisError::TooFew {
                what: "objectives",
                count: 0,
                minimum: 1,
            });
        }
        if objectives > MAX_OBJECTIVES {
            return Err(AnalysisError::Count {
                what: "objectives",
                count: objectives,
                limit: MAX_OBJECTIVES,
            });
        }
        for v in &variables {
            if !v.low().is_finite() {
                return Err(AnalysisError::Domain {
                    what: "variable's low bound (NSGA-II needs two finite bounds)",
                    value: v.low(),
                });
            }
            if !v.high().is_finite() {
                return Err(AnalysisError::Domain {
                    what: "variable's high bound (NSGA-II needs two finite bounds)",
                    value: v.high(),
                });
            }
            // Within ±f64::MAX/4, crossover's sums and spreads can't overflow.
            for bound in [v.low(), v.high()] {
                if bound.abs() > MAX_BOUND {
                    return Err(AnalysisError::Domain {
                        what: "variable's bound (NSGA-II takes bounds within ±f64::MAX/4)",
                        value: bound,
                    });
                }
            }
            if v.is_integer() {
                return Err(AnalysisError::Unsupported(format!(
                    "integer variable {:?} in NSGA-II",
                    v.name()
                )));
            }
        }
        // Cast: n ≤ 200, exact in f64.
        let mutation_probability = 1.0 / variables.len() as f64;
        Ok(Self {
            variables,
            objectives,
            population: 100,
            generations: 250,
            crossover_probability: 0.9,
            crossover_index: 20.0,
            mutation_probability,
            mutation_index: 20.0,
        })
    }

    /// The same optimizer with a population of `size`: an even number, as children are bred two
    /// at a time, from 4 to [`MAX_POPULATION`].
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] below 4, [`AnalysisError::Count`] above [`MAX_POPULATION`],
    /// and [`AnalysisError::Domain`] for an odd size.
    pub fn with_population(mut self, size: usize) -> Result<Self, AnalysisError> {
        if size < 4 {
            return Err(AnalysisError::TooFew {
                what: "population",
                count: size,
                minimum: 4,
            });
        }
        if size > MAX_POPULATION {
            return Err(AnalysisError::Count {
                what: "population",
                count: size,
                limit: MAX_POPULATION,
            });
        }
        if !size.is_multiple_of(2) {
            return Err(AnalysisError::Domain {
                what: "population (an even number)",
                // Cast: at most 2¹², exact in f64.
                value: size as f64,
            });
        }
        self.population = size;
        Ok(self)
    }

    /// The same optimizer, run for `generations` generations, the first, random one included: it
    /// evaluates the population that many times.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] for none.
    pub fn with_generations(mut self, generations: usize) -> Result<Self, AnalysisError> {
        if generations == 0 {
            return Err(AnalysisError::TooFew {
                what: "generations",
                count: 0,
                minimum: 1,
            });
        }
        self.generations = generations;
        Ok(self)
    }

    /// The same optimizer with SBX crossing a pair of parents with `probability`, and
    /// distribution index `index`: the larger, the closer children fall to their parents.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a probability outside `[0, 1]`, or an index that is negative
    /// or not finite.
    pub fn with_crossover(mut self, probability: f64, index: f64) -> Result<Self, AnalysisError> {
        self.crossover_probability = check_probability("crossover probability", probability)?;
        self.crossover_index = check_index("crossover distribution index", index)?;
        Ok(self)
    }

    /// The same optimizer with polynomial mutation changing each variable of a child with
    /// `probability`, and distribution index `index`: the larger, the smaller the changes.
    ///
    /// # Errors
    ///
    /// As [`Nsga2::with_crossover`].
    pub fn with_mutation(mut self, probability: f64, index: f64) -> Result<Self, AnalysisError> {
        self.mutation_probability = check_probability("mutation probability", probability)?;
        self.mutation_index = check_index("mutation distribution index", index)?;
        Ok(self)
    }

    /// The variables.
    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    /// The number of goals.
    pub fn objectives(&self) -> usize {
        self.objectives
    }

    /// The population's size.
    pub fn population(&self) -> usize {
        self.population
    }

    /// The number of generations.
    pub fn generations(&self) -> usize {
        self.generations
    }

    /// Starts a run from `seed`: its first candidates are the random first population.
    pub fn start(&self, seed: u64) -> Run {
        let mut rng = SeededRng::for_stream(seed, &[0]);
        let candidates = (0..self.population)
            .map(|_| {
                self.variables
                    .iter()
                    .map(|v| v.low() + rng.uniform() * (v.high() - v.low()))
                    .collect()
            })
            .collect();
        Run {
            settings: self.clone(),
            seed,
            generation: 0,
            members: Vec::new(),
            candidates,
        }
    }

    /// Runs NSGA-II on `model` under constraints from `seed`, evaluating each candidate in turn.
    ///
    /// # Errors
    ///
    /// [`Run::tell_constrained`]'s.
    pub fn minimize_constrained(
        &self,
        seed: u64,
        mut model: impl FnMut(&[f64]) -> Goals,
    ) -> Result<Front, AnalysisError> {
        let mut run = self.start(seed);
        loop {
            let goals: Vec<Goals> = run.candidates().iter().map(|x| model(x)).collect();
            if let Some(front) = run.tell_constrained(&goals)? {
                return Ok(front);
            }
        }
    }

    /// Runs NSGA-II on `model` from `seed`, evaluating each candidate in turn: the model gives
    /// each candidate's goals, every one to be made as small as it can be.
    ///
    /// # Errors
    ///
    /// [`Run::tell`]'s.
    pub fn minimize(
        &self,
        seed: u64,
        mut model: impl FnMut(&[f64]) -> Vec<f64>,
    ) -> Result<Front, AnalysisError> {
        let mut run = self.start(seed);
        loop {
            let values: Vec<Vec<f64>> = run.candidates().iter().map(|x| model(x)).collect();
            if let Some(front) = run.tell(&values)? {
                return Ok(front);
            }
        }
    }
}

/// A run in progress: its population, ranked, and the candidates waiting to be evaluated.
#[derive(Debug, Clone)]
pub struct Run {
    settings: Nsga2,
    seed: u64,
    /// Generations evaluated so far.
    generation: usize,
    /// The population, ranked: empty before the first generation is told.
    members: Vec<Member>,
    /// The candidates waiting to be evaluated: empty once the run has finished.
    candidates: Vec<Vec<f64>>,
}

impl Run {
    /// The candidates to evaluate next: the first population, then each generation's children.
    /// Empty once the run has finished.
    pub fn candidates(&self) -> &[Vec<f64>] {
        &self.candidates
    }

    /// How many generations have been evaluated.
    pub fn generation(&self) -> usize {
        self.generation
    }

    /// The population as it stands, ranked: empty before the first generation is told.
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// Takes the goals of the candidates, in their order, with no constraints: the population
    /// is updated, and once the last generation is told the run's [`Front`] is returned. A
    /// design with a goal of `+∞` counts as failed ([`Goals::failed`]): every design whose goals
    /// are all finite dominates it.
    ///
    /// # Errors
    ///
    /// [`Run::tell_constrained`]'s.
    pub fn tell(&mut self, values: &[Vec<f64>]) -> Result<Option<Front>, AnalysisError> {
        let goals: Vec<Goals> = values.iter().cloned().map(Goals::feasible).collect();
        self.tell_constrained(&goals)
    }

    /// [`Run::tell`] with each candidate's constraint violation: designs are sorted by
    /// constrained domination. A failed design ([`Goals::failed`]) may have any number of goals,
    /// as they are set aside: each counts as `+∞`. A design with a goal of `+∞` fails too, its
    /// violation taken as `+∞`.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Length`] for a number of goals other than the candidates' (telling a
    /// finished run, which has none, too) or a design's number of goals other than the
    /// optimizer's; [`AnalysisError::Output`] for a goal that is NaN or `−∞`, with the index of
    /// its evaluation in the run; and [`AnalysisError::Domain`] for a violation that is negative
    /// or NaN.
    pub fn tell_constrained(&mut self, goals: &[Goals]) -> Result<Option<Front>, AnalysisError> {
        let m = self.settings.objectives;
        if goals.len() != self.candidates.len() || self.candidates.is_empty() {
            return Err(AnalysisError::Length {
                what: "goals, against the generation's candidates",
                length: goals.len(),
                expected: self.candidates.len(),
            });
        }
        let evaluated = self.generation * self.settings.population;
        let mut children = Vec::with_capacity(goals.len());
        for (k, (point, g)) in self.candidates.iter().zip(goals).enumerate() {
            if g.violation.is_nan() || g.violation < 0.0 {
                return Err(AnalysisError::Domain {
                    what: "constraint violation (must not be negative or NaN)",
                    value: g.violation,
                });
            }
            // A failed design's goals are set aside; any other's are checked, and one with a goal
            // of +∞ fails.
            let mut failed = g.violation == f64::INFINITY;
            if !failed {
                if g.objectives.len() != m {
                    return Err(AnalysisError::Length {
                        what: "a design's goals",
                        length: g.objectives.len(),
                        expected: m,
                    });
                }
                if let Some(&value) = g
                    .objectives
                    .iter()
                    .find(|v| v.is_nan() || **v == f64::NEG_INFINITY)
                {
                    return Err(AnalysisError::Output {
                        index: evaluated + k,
                        value,
                    });
                }
                failed = g.objectives.contains(&f64::INFINITY);
            }
            let objectives = if failed {
                vec![f64::INFINITY; m]
            } else {
                g.objectives.clone()
            };
            children.push(Member {
                point: point.clone(),
                objectives,
                violation: if failed { f64::INFINITY } else { g.violation },
                rank: 0,
                crowding: 0.0,
            });
        }
        let mut combined = std::mem::take(&mut self.members);
        combined.extend(children);
        self.members = survivors(combined, self.settings.population);
        self.generation += 1;
        if self.generation == self.settings.generations {
            self.candidates.clear();
            let members = self
                .members
                .iter()
                .filter(|m| m.rank == 0)
                .cloned()
                .collect();
            return Ok(Some(Front {
                members,
                population: self.members.clone(),
                evaluations: self.generation * self.settings.population,
                generations: self.generation,
            }));
        }
        self.candidates = self.breed();
        Ok(None)
    }

    /// The next generation's children: parents by binary tournaments, crossed in pairs by SBX,
    /// then mutated.
    ///
    /// The tournaments' contestants are two shuffles of the population laid end to end and taken
    /// two at a time, so every design plays exactly two tournaments, as in Deb's NSGA-II code and
    /// pymoo; the paper gives no pairing. The population is even, so no tournament pits a design
    /// against itself. Winners are paired in turn: the first two, the next two, and so on.
    fn breed(&self) -> Vec<Vec<f64>> {
        let s = &self.settings;
        // Cast: a generation count fits in u64.
        let mut rng = SeededRng::for_stream(self.seed, &[self.generation as u64]);
        let contestants = contestants(&mut rng, self.members.len());
        let parents: Vec<&[f64]> = contestants
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[i, j]| self.tournament(i, j, &mut rng))
            .collect();
        let mut children = Vec::with_capacity(s.population);
        for &[a, b] in parents.as_chunks::<2>().0 {
            let (mut c1, mut c2) = (a.to_vec(), b.to_vec());
            if rng.uniform() < s.crossover_probability {
                for (i, v) in s.variables.iter().enumerate() {
                    if rng.uniform() < 0.5 {
                        let (y1, y2) =
                            sbx(a[i], b[i], v.low(), v.high(), s.crossover_index, &mut rng);
                        (c1[i], c2[i]) = (y1, y2);
                    }
                }
            }
            for child in [&mut c1, &mut c2] {
                for (i, v) in s.variables.iter().enumerate() {
                    if rng.uniform() < s.mutation_probability {
                        child[i] = polynomial_mutation(
                            child[i],
                            v.low(),
                            v.high(),
                            s.mutation_index,
                            &mut rng,
                        );
                    }
                }
            }
            children.push(c1);
            children.push(c2);
        }
        children
    }

    /// A binary tournament between members `i` and `j`: the winner by the crowded comparison, a
    /// tie to either with equal chance.
    fn tournament(&self, i: usize, j: usize, rng: &mut SeededRng) -> &[f64] {
        let (a, b) = (&self.members[i], &self.members[j]);
        let winner = match crowded_comparison(a, b) {
            std::cmp::Ordering::Less => a,
            std::cmp::Ordering::Greater => b,
            std::cmp::Ordering::Equal => {
                if rng.uniform() < 0.5 {
                    a
                } else {
                    b
                }
            }
        };
        &winner.point
    }
}

/// The tournaments' contestants for a population of `n`: two shuffles of `0..n` (Fisher–Yates,
/// from the end) laid end to end, to be taken two at a time.
fn contestants(rng: &mut SeededRng, n: usize) -> Vec<usize> {
    let mut contestants = Vec::with_capacity(2 * n);
    for _ in 0..2 {
        let mut shuffle: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            shuffle.swap(i, draw_index(rng, i + 1));
        }
        contestants.extend(shuffle);
    }
    contestants
}

/// A uniform index below `n` (`n ≥ 1`): `⌊u n⌋` for a uniform `u` on `[0, 1)`, held below `n`.
/// Its bias is at most `n/2⁵³`.
fn draw_index(rng: &mut SeededRng, n: usize) -> usize {
    // Casts: n ≤ 2¹², exact in f64, and ⌊u n⌋ < n converts back exactly.
    let k = (rng.uniform() * n as f64).floor() as usize;
    k.min(n - 1)
}

/// The crowded comparison: [`Less`](std::cmp::Ordering::Less) if `a` ranks ahead of `b`.
fn crowded_comparison(a: &Member, b: &Member) -> std::cmp::Ordering {
    a.rank
        .cmp(&b.rank)
        .then_with(|| b.crowding.total_cmp(&a.crowding))
}

/// Whether `a`'s goals dominate `b`'s: no worse in any, better in at least one. The two should
/// be the same length; only as many goals as the shorter has are compared.
pub fn dominates(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y) && a.iter().zip(b).any(|(x, y)| x < y)
}

/// Constrained domination (Deb et al. 2002, §VI).
fn constrained_dominates(a: &Member, b: &Member) -> bool {
    let (fa, fb) = (a.violation == 0.0, b.violation == 0.0);
    match (fa, fb) {
        (true, false) => true,
        (false, true) => false,
        (false, false) => a.violation < b.violation,
        (true, true) => dominates(&a.objectives, &b.objectives),
    }
}

/// Fast non-dominated sorting: each member's front, counted from 0, as indices into `members`,
/// each front in the members' order.
fn fronts(members: &[Member]) -> Vec<Vec<usize>> {
    let n = members.len();
    let mut dominated: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut count = vec![0usize; n];
    for p in 0..n {
        for q in p + 1..n {
            if constrained_dominates(&members[p], &members[q]) {
                dominated[p].push(q);
                count[q] += 1;
            } else if constrained_dominates(&members[q], &members[p]) {
                dominated[q].push(p);
                count[p] += 1;
            }
        }
    }
    let mut fronts = Vec::new();
    let mut current: Vec<usize> = (0..n).filter(|&p| count[p] == 0).collect();
    while !current.is_empty() {
        let mut next = Vec::new();
        for &p in &current {
            for &q in &dominated[p] {
                count[q] -= 1;
                if count[q] == 0 {
                    next.push(q);
                }
            }
        }
        next.sort_unstable();
        fronts.push(current);
        current = next;
    }
    fronts
}

/// The crowding distance of each design of a front, given each design's goals, in the front's
/// order. A goal whose range in the front is zero or not finite adds nothing between the ends.
fn crowding(front: &[&[f64]]) -> Vec<f64> {
    let l = front.len();
    let mut distance = vec![0.0; l];
    let Some(first) = front.first() else {
        return distance;
    };
    #[allow(
        clippy::needless_range_loop,
        reason = "`m` indexes each design's goals, not `front`"
    )]
    for m in 0..first.len() {
        let value = |k: usize| front[k][m];
        let mut order: Vec<usize> = (0..l).collect();
        // A stable sort: ties keep the front's order.
        order.sort_by(|&a, &b| value(a).total_cmp(&value(b)));
        distance[order[0]] = f64::INFINITY;
        distance[order[l - 1]] = f64::INFINITY;
        let range = value(order[l - 1]) - value(order[0]);
        if !(range.is_finite() && range > 0.0) {
            continue;
        }
        for k in 1..l.saturating_sub(1) {
            distance[order[k]] += (value(order[k + 1]) - value(order[k - 1])) / range;
        }
    }
    distance
}

/// The `n` survivors of `combined` (parents then children): whole fronts in rank order, and the
/// front that doesn't fit cut by crowding distance, largest first. Each keeps the rank and
/// crowding distance of the sort of `combined`.
fn survivors(combined: Vec<Member>, n: usize) -> Vec<Member> {
    let fronts = fronts(&combined);
    let mut ranked: Vec<Option<Member>> = combined.into_iter().map(Some).collect();
    let mut next = Vec::with_capacity(n);
    for (rank, front) in fronts.iter().enumerate() {
        if next.len() == n {
            break;
        }
        let goals: Vec<&[f64]> = front
            .iter()
            .filter_map(|&i| ranked[i].as_ref().map(|m| m.objectives.as_slice()))
            .collect();
        let distance = crowding(&goals);
        let mut order: Vec<usize> = (0..front.len()).collect();
        if next.len() + front.len() > n {
            // A stable sort: equal distances keep the front's order.
            order.sort_by(|&a, &b| distance[b].total_cmp(&distance[a]));
            order.truncate(n - next.len());
            order.sort_unstable();
        }
        for k in order {
            if let Some(mut member) = ranked[front[k]].take() {
                member.rank = rank;
                member.crowding = distance[k];
                next.push(member);
            }
        }
    }
    next
}

/// Bounded simulated binary crossover of one variable: the two children of parent values `x1`
/// and `x2` within `[low, high]`, the children swapped with probability ½. Parents closer than
/// 10⁻¹⁴ are copied.
///
/// With `y₁ < y₂` the parents and `u` uniform on `[0, 1)`, unbounded SBX's spread for a child is
/// `β_q = (2u)^(1/(η+1))` for `u ≤ ½`, else `(1/(2(1 − u)))^(1/(η+1))`, the inverse of the spread's
/// distribution (Deb and Beyer 1999, eq. 4). A child `c₁ = ½((y₁ + y₂) − β (y₂ − y₁))` stays
/// above `low` while `β ≤ β_b = 1 + 2(y₁ − low)/(y₂ − y₁)`, a spread whose probability is
/// `α/2` with `α = 2 − β_b^−(η+1)`. Drawing the spread with probability scaled to that, at
/// `u′ = u α/2`, gives `β_q = (u α)^(1/(η+1))` for `u ≤ 1/α`, else `(1/(2 − u α))^(1/(η+1))`; the
/// other child uses the same `u` with `β_b = 1 + 2(high − y₂)/(y₂ − y₁)`. The clamp only catches
/// rounding.
fn sbx(x1: f64, x2: f64, low: f64, high: f64, eta: f64, rng: &mut SeededRng) -> (f64, f64) {
    if (x1 - x2).abs() <= 1e-14 {
        return (x1, x2);
    }
    let (y1, y2) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
    let u = rng.uniform();
    let exponent = 1.0 / (eta + 1.0);
    // β_q for a child whose spread is cut at `beta`, the ratio its bound allows.
    let beta_q = |beta: f64| {
        let alpha = 2.0 - beta.powf(-(eta + 1.0));
        if u <= 1.0 / alpha {
            (u * alpha).powf(exponent)
        } else {
            (1.0 / (2.0 - u * alpha)).powf(exponent)
        }
    };
    let gap = y2 - y1;
    let c1 = 0.5 * ((y1 + y2) - beta_q(1.0 + 2.0 * (y1 - low) / gap) * gap);
    let c2 = 0.5 * ((y1 + y2) + beta_q(1.0 + 2.0 * (high - y2) / gap) * gap);
    let (c1, c2) = (c1.clamp(low, high), c2.clamp(low, high));
    if rng.uniform() < 0.5 {
        (c2, c1)
    } else {
        (c1, c2)
    }
}

/// Bounded polynomial mutation of one value `y` within `[low, high]`: `y + δ_q (high − low)`.
///
/// Unbounded, the step `δ` has density `½(η + 1)(1 − |δ|)^η` on `[−1, 1]`, half each way. Here
/// each half is cut at its bound, `δ₁ = (y − low)/(high − low)` below and `δ₂ = (high − y)/(high −
/// low)` above, and keeps its half of the probability: for `u ≤ ½`,
/// `δ_q = (2u + (1 − 2u)(1 − δ₁)^(η+1))^(1/(η+1)) − 1`, which runs from `−δ₁` at `u = 0` to 0 at
/// `u = ½`; for `u > ½`, `δ_q = 1 − (2(1 − u) + 2(u − ½)(1 − δ₂)^(η+1))^(1/(η+1))`, from 0 to
/// `δ₂`. The clamp only catches rounding.
fn polynomial_mutation(y: f64, low: f64, high: f64, eta: f64, rng: &mut SeededRng) -> f64 {
    let range = high - low;
    let delta1 = (y - low) / range;
    let delta2 = (high - y) / range;
    let u = rng.uniform();
    let power = 1.0 / (eta + 1.0);
    let delta_q = if u <= 0.5 {
        let value = 2.0 * u + (1.0 - 2.0 * u) * (1.0 - delta1).powf(eta + 1.0);
        value.powf(power) - 1.0
    } else {
        let value = 2.0 * (1.0 - u) + 2.0 * (u - 0.5) * (1.0 - delta2).powf(eta + 1.0);
        1.0 - value.powf(power)
    };
    (y + delta_q * range).clamp(low, high)
}

/// The generational distance of `set` from `reference`: the mean, over the points of `set`, of
/// the Euclidean distance to the nearest point of `reference`. It is Deb et al.'s (2002)
/// convergence metric Υ (§IV-B, p. 188), and pymoo's `GD`: zero when every point lies on the
/// reference. NaN for an empty `set` or `reference`.
pub fn generational_distance(set: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    mean_nearest(set, reference)
}

/// The inverted generational distance: the mean, over the points of `reference`, of the
/// Euclidean distance to the nearest point of `set`, as pymoo's `IGD`. Small only if `set` both
/// lies near the reference and covers all of it.
pub fn inverted_generational_distance(set: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    mean_nearest(reference, set)
}

/// The mean over `from` of the distance to the nearest point of `to`.
fn mean_nearest(from: &[Vec<f64>], to: &[Vec<f64>]) -> f64 {
    if from.is_empty() || to.is_empty() {
        return f64::NAN;
    }
    let total = from
        .iter()
        .map(|p| {
            to.iter()
                .map(|q| {
                    p.iter()
                        .zip(q)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        .sqrt()
                })
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, |total, d| total + d);
    // Cast: a count of points, exact in f64 below 2⁵³.
    total / from.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(objectives: &[f64], violation: f64) -> Member {
        Member {
            point: Vec::new(),
            objectives: objectives.to_vec(),
            violation,
            rank: 0,
            crowding: 0.0,
        }
    }

    fn unit(name: &str) -> Variable {
        Variable::new(name, 0.5, 0.1)
            .unwrap()
            .within(0.0, 1.0)
            .unwrap()
    }

    #[test]
    fn domination_needs_one_strictly_better() {
        assert!(dominates(&[1.0, 2.0], &[1.0, 3.0]));
        assert!(!dominates(&[1.0, 2.0], &[1.0, 2.0]), "equal");
        assert!(!dominates(&[1.0, 3.0], &[2.0, 2.0]), "a trade-off");
        assert!(dominates(&[1.0, 2.0], &[f64::INFINITY, 2.0]));
        // Constrained: feasible beats infeasible whatever the goals; two infeasible go by violation.
        assert!(constrained_dominates(
            &member(&[9.0, 9.0], 0.0),
            &member(&[0.0, 0.0], 0.1)
        ));
        assert!(constrained_dominates(
            &member(&[9.0, 9.0], 0.1),
            &member(&[0.0, 0.0], 0.2)
        ));
        assert!(!constrained_dominates(
            &member(&[0.0, 0.0], 0.2),
            &member(&[9.0, 9.0], 0.2)
        ));
    }

    /// Fronts of a hand-made set: (1, 5), (2, 3) and (4, 1) trade off; (3, 4) is dominated by
    /// (2, 3) only, and (5, 5) by every other.
    #[test]
    fn fronts_peel_in_order() {
        let members: Vec<Member> = [[3.0, 4.0], [1.0, 5.0], [5.0, 5.0], [2.0, 3.0], [4.0, 1.0]]
            .iter()
            .map(|f| member(f, 0.0))
            .collect();
        assert_eq!(fronts(&members), vec![vec![1, 3, 4], vec![0], vec![2]]);
        // Failed designs form the last front.
        let failed = member(&[f64::INFINITY, f64::INFINITY], f64::INFINITY);
        let mut with_failed = members.clone();
        with_failed.push(failed.clone());
        with_failed.push(failed);
        assert_eq!(
            fronts(&with_failed),
            vec![vec![1, 3, 4], vec![0], vec![2], vec![5, 6]]
        );
    }

    /// Crowding distances by hand: four points on a line, goal ranges 3 and 6.
    #[test]
    fn crowding_by_hand() {
        let goals: [&[f64]; 4] = [&[0.0, 6.0], &[1.0, 4.0], &[3.0, 0.0], &[2.0, 3.0]];
        let d = crowding(&goals);
        assert_eq!(d[0], f64::INFINITY);
        assert_eq!(d[2], f64::INFINITY);
        // (1, 4): neighbours (0, 6) and (2, 3): 2/3 + 3/6.
        assert!((d[1] - (2.0 / 3.0 + 3.0 / 6.0)).abs() <= 1e-15);
        // (2, 3): neighbours (1, 4) and (3, 0): 2/3 + 4/6.
        assert!((d[3] - (2.0 / 3.0 + 4.0 / 6.0)).abs() <= 1e-15);
        // Two or fewer points are all ends; a goal of zero range adds nothing.
        assert_eq!(
            crowding(&[&[1.0, 2.0], &[2.0, 1.0]]),
            vec![f64::INFINITY; 2]
        );
        let flat: [&[f64]; 3] = [&[1.0, 5.0], &[2.0, 5.0], &[3.0, 5.0]];
        assert_eq!(crowding(&flat)[1], 1.0);
        assert!(crowding(&[]).is_empty());
    }

    /// The front that doesn't fit is cut by crowding distance, its ends kept.
    #[test]
    fn survivors_cut_the_last_front_by_crowding() {
        let combined: Vec<Member> = [
            [0.0, 1.0],
            [0.1, 0.9],
            [0.5, 0.5],
            [0.55, 0.45],
            [1.0, 0.0],
            [2.0, 2.0],
        ]
        .iter()
        .map(|f| member(f, 0.0))
        .collect();
        let next = survivors(combined, 3);
        let kept: Vec<Vec<f64>> = next.iter().map(|m| m.objectives.clone()).collect();
        // The two ends, and of the middle three the one with the widest gaps around it: (0.1, 0.9)
        // and (0.55, 0.45) have 0.5 + 0.5, (0.5, 0.5) 0.45 + 0.45. The tie goes to the earlier.
        assert_eq!(kept, vec![vec![0.0, 1.0], vec![0.1, 0.9], vec![1.0, 0.0]]);
        assert!(next.iter().all(|m| m.rank == 0));
    }

    /// SBX's spread factor `β = |c₂ − c₁| / |x₂ − x₁|` far from the bounds follows Deb and
    /// Agrawal's density: `P(β ≤ b) = bⁿ⁺¹/2` for `b ≤ 1`, and `1 − b^−(η+1)/2` above.
    #[test]
    fn sbx_spread_follows_its_density() {
        let eta = 2.0;
        let draws = 100_000;
        let mut rng = SeededRng::seed_from_u64(1);
        let betas: Vec<f64> = (0..draws)
            .map(|_| {
                let (c1, c2) = sbx(-0.5, 0.5, -1e9, 1e9, eta, &mut rng);
                (c2 - c1).abs()
            })
            .collect();
        for b in [0.25_f64, 0.5, 0.8, 1.0, 1.5, 3.0] {
            let expected: f64 = if b <= 1.0 {
                0.5 * b.powf(eta + 1.0)
            } else {
                1.0 - 0.5 * b.powf(-(eta + 1.0))
            };
            let share = betas.iter().filter(|&&x| x <= b).count() as f64 / draws as f64;
            let sigma = (expected * (1.0 - expected) / draws as f64).sqrt();
            assert!(
                (share - expected).abs() <= 5.0 * sigma + 1e-6,
                "P(β ≤ {b}) {share} against {expected}"
            );
        }
    }

    /// Near a bound, SBX's children and mutated values stay within it, and their spread is cut,
    /// not piled at it: none lands on the bound.
    #[test]
    fn sbx_and_mutation_stay_within_bounds() {
        let mut rng = SeededRng::seed_from_u64(2);
        let mut at_bound = 0;
        for _ in 0..20_000 {
            let (c1, c2) = sbx(0.001, 0.3, 0.0, 1.0, 20.0, &mut rng);
            assert!((0.0..=1.0).contains(&c1) && (0.0..=1.0).contains(&c2));
            at_bound += usize::from(c1 == 0.0 || c2 == 0.0);
            let (c1, c2) = sbx(0.7, 0.999, 0.0, 1.0, 20.0, &mut rng);
            assert!((0.0..=1.0).contains(&c1) && (0.0..=1.0).contains(&c2));
            at_bound += usize::from(c1 == 1.0 || c2 == 1.0);
            let y = polynomial_mutation(0.999, 0.0, 1.0, 20.0, &mut rng);
            assert!((0.0..=1.0).contains(&y));
            at_bound += usize::from(y == 1.0);
            let y = polynomial_mutation(0.001, 0.0, 1.0, 20.0, &mut rng);
            assert!((0.0..=1.0).contains(&y));
            at_bound += usize::from(y == 0.0);
        }
        assert_eq!(at_bound, 0);
        // Parents within 1e-14 are copied.
        assert_eq!(sbx(0.3, 0.3, 0.0, 1.0, 20.0, &mut rng), (0.3, 0.3));
    }

    /// Polynomial mutation's step `δ_q`, cut at each side's own bound, follows its distribution:
    /// at `y = 0.2` in `[0, 1]`, `δ₁ = 0.2` below and `δ₂ = 0.8` above, and with
    /// `tᵢ = (1 − δᵢ)^(η+1)`, `P(δ_q ≤ −d) = ((1 − d)^(η+1) − t₁) / (2 (1 − t₁))` for `d ≤ δ₁`, and
    /// `P(δ_q ≥ d) = ((1 − d)^(η+1) − t₂) / (2 (1 − t₂))` for `d ≤ δ₂`; half the steps each way.
    #[test]
    fn mutation_step_follows_its_distribution() {
        let draws = 100_000;
        for eta in [5.0_f64, 20.0] {
            let mut rng = SeededRng::seed_from_u64(3);
            let steps: Vec<f64> = (0..draws)
                .map(|_| polynomial_mutation(0.2, 0.0, 1.0, eta, &mut rng) - 0.2)
                .collect();
            let share = |f: &dyn Fn(f64) -> bool| {
                steps.iter().filter(|&&x| f(x)).count() as f64 / draws as f64
            };
            let check = |measured: f64, expected: f64, what: &str| {
                let sigma = (expected * (1.0 - expected) / draws as f64).sqrt();
                assert!(
                    (measured - expected).abs() <= 5.0 * sigma + 1e-6,
                    "η = {eta}, {what}: {measured} against {expected}"
                );
            };
            let (t1, t2) = (0.8_f64.powf(eta + 1.0), 0.2_f64.powf(eta + 1.0));
            for d in [0.01_f64, 0.03, 0.1, 0.19] {
                let below = ((1.0 - d).powf(eta + 1.0) - t1) / (2.0 * (1.0 - t1));
                check(share(&|x| x <= -d), below, &format!("P(δ ≤ −{d})"));
                let above = ((1.0 - d).powf(eta + 1.0) - t2) / (2.0 * (1.0 - t2));
                check(share(&|x| x >= d), above, &format!("P(δ ≥ {d})"));
            }
            check(share(&|x| x < 0.0), 0.5, "P(δ < 0)");
        }
    }

    /// Near both bounds, each SBX child's spread is cut at its own bound: the lower child's
    /// `β = (y₁ + y₂ − 2c)/(y₂ − y₁)` at `β_b = 1 + 2(y₁ − low)/(y₂ − y₁)`, the upper child's
    /// `β = (2c − y₁ − y₂)/(y₂ − y₁)` at `β_b = 1 + 2(high − y₂)/(y₂ − y₁)`, each scaled by `1/α`,
    /// `α = 2 − β_b^−(η+1)`: `P(β ≤ b) = b^(η+1)/α` for `b ≤ 1`, and `(2 − b^−(η+1))/α` from 1 to
    /// `β_b`. Here the two cuts differ, 1.4 below and 1.8 above. The children come out in either
    /// order with equal chance.
    #[test]
    fn sbx_spread_is_cut_at_each_bound() {
        let (eta, draws) = (2.0_f64, 100_000);
        let (y1, y2, low, high) = (0.05_f64, 0.3_f64, 0.0_f64, 0.4_f64);
        let gap = y2 - y1;
        let mut rng = SeededRng::seed_from_u64(4);
        let mut lower_first = 0;
        let (mut lower, mut upper) = (Vec::new(), Vec::new());
        for _ in 0..draws {
            let (c1, c2) = sbx(y1, y2, low, high, eta, &mut rng);
            lower_first += usize::from(c1 < c2);
            lower.push((y1 + y2 - 2.0 * c1.min(c2)) / gap);
            upper.push((2.0 * c1.max(c2) - y1 - y2) / gap);
        }
        for (betas, beta_b, side) in [
            (&lower, 1.0 + 2.0 * (y1 - low) / gap, "lower"),
            (&upper, 1.0 + 2.0 * (high - y2) / gap, "upper"),
        ] {
            let alpha = 2.0 - beta_b.powf(-(eta + 1.0));
            for b in [
                0.5, 0.9, 0.94, 0.945, 0.96, 0.98, 1.0, 1.1, 1.3, 1.39, 1.6, 1.79,
            ] {
                if b > beta_b {
                    continue;
                }
                let expected = if b <= 1.0 {
                    b.powf(eta + 1.0) / alpha
                } else {
                    (2.0 - b.powf(-(eta + 1.0))) / alpha
                };
                let share = betas.iter().filter(|&&x| x <= b).count() as f64 / draws as f64;
                let sigma = (expected * (1.0 - expected) / draws as f64).sqrt();
                assert!(
                    (share - expected).abs() <= 5.0 * sigma + 1e-6,
                    "{side} child: P(β ≤ {b}) {share} against {expected}"
                );
            }
            assert!(betas.iter().all(|&b| b <= beta_b * (1.0 + 1e-12)));
        }
        let half = lower_first as f64 / draws as f64;
        assert!(
            (half - 0.5).abs() <= 5.0 * (0.25 / draws as f64).sqrt(),
            "{half}"
        );
    }

    /// Two shuffles end to end: every design plays exactly two tournaments, never against itself.
    #[test]
    fn every_design_plays_two_tournaments() {
        for n in [4, 6, 20, 100] {
            for seed in 0..20 {
                let mut rng = SeededRng::seed_from_u64(seed);
                let c = contestants(&mut rng, n);
                assert_eq!(c.len(), 2 * n);
                for i in 0..n {
                    assert_eq!(c.iter().filter(|&&k| k == i).count(), 2);
                }
                assert!(c.as_chunks::<2>().0.iter().all(|[a, b]| a != b));
            }
        }
        // Each shuffle is uniform: over many, each design is first a quarter of the time (n = 4).
        let mut rng = SeededRng::seed_from_u64(9);
        let mut first = [0_usize; 4];
        for _ in 0..40_000 {
            first[contestants(&mut rng, 4)[0]] += 1;
        }
        for count in first {
            let share = count as f64 / 40_000.0;
            assert!(
                (share - 0.25).abs() <= 5.0 * (0.1875_f64 / 40_000.0).sqrt(),
                "{first:?}"
            );
        }
    }

    /// A design with a goal of +∞ fails: violation +∞, behind even a design that breaks a limit;
    /// a generation of only such designs leaves no feasible front.
    #[test]
    fn infinite_goals_fail() {
        let n = Nsga2::new(vec![unit("x")], 2)
            .unwrap()
            .with_population(4)
            .unwrap()
            .with_generations(1)
            .unwrap();
        let mut run = n.start(1);
        let goals = vec![
            Goals::feasible(vec![f64::INFINITY, -100.0]),
            Goals::constrained(vec![1.0, 1.0], &[0.5]),
            Goals::feasible(vec![2.0, 2.0]),
            Goals::feasible(vec![3.0, 1.0]),
        ];
        let front = run.tell_constrained(&goals).unwrap().unwrap();
        let members = run.members();
        assert_eq!(members.len(), 4);
        let infinite = members
            .iter()
            .find(|m| m.violation == f64::INFINITY)
            .unwrap();
        let broken = members.iter().find(|m| m.violation == 0.5).unwrap();
        assert_eq!(infinite.objectives, vec![f64::INFINITY; 2]);
        assert!(infinite.rank > broken.rank);
        assert!(front.is_feasible() && front.members.len() == 2);
        let mut run = n.start(2);
        let all = vec![vec![f64::INFINITY, 0.0]; 4];
        let front = run.tell(&all).unwrap().unwrap();
        assert!(!front.is_feasible());
    }

    #[test]
    fn settings_are_checked() {
        let x = || vec![unit("x")];
        assert!(Nsga2::new(x(), 0).is_err());
        assert!(Nsga2::new(x(), MAX_OBJECTIVES + 1).is_err());
        assert!(Nsga2::new(vec![], 2).is_err());
        let open = Variable::new("x", 0.0, 1.0).unwrap();
        assert!(matches!(
            Nsga2::new(vec![open.clone()], 2),
            Err(AnalysisError::Domain { what, .. }) if what.contains("low bound")
        ));
        let half = open.within(0.0, f64::INFINITY).unwrap();
        assert!(matches!(
            Nsga2::new(vec![half], 2),
            Err(AnalysisError::Domain { what, .. }) if what.contains("high bound")
        ));
        let whole = Variable::new("k", 1.0, 1.0)
            .unwrap()
            .within(0.0, 4.0)
            .unwrap()
            .integer()
            .unwrap();
        assert!(matches!(
            Nsga2::new(vec![whole], 2),
            Err(AnalysisError::Unsupported(_))
        ));
        let n = Nsga2::new(x(), 2).unwrap();
        assert!(matches!(
            n.clone().with_population(5),
            Err(AnalysisError::Domain { .. })
        ));
        assert!(matches!(
            n.clone().with_population(2),
            Err(AnalysisError::TooFew { .. })
        ));
        assert!(matches!(
            n.clone().with_population(MAX_POPULATION + 2),
            Err(AnalysisError::Count { .. })
        ));
        assert!(n.clone().with_population(MAX_POPULATION).is_ok());
        let wide = Variable::new("x", 0.0, 1.0)
            .unwrap()
            .within(-f64::MAX, f64::MAX)
            .unwrap();
        assert!(matches!(
            Nsga2::new(vec![wide], 2),
            Err(AnalysisError::Domain { what, .. }) if what.contains("MAX/4")
        ));
        assert!(n.clone().with_generations(0).is_err());
        assert!(n.clone().with_crossover(1.1, 20.0).is_err());
        assert!(n.clone().with_crossover(f64::NAN, 20.0).is_err());
        assert!(n.clone().with_mutation(0.5, -1.0).is_err());
        assert!(n.clone().with_mutation(0.5, f64::INFINITY).is_err());
        let n = n.with_population(4).unwrap().with_generations(3).unwrap();
        assert_eq!((n.population(), n.generations(), n.objectives()), (4, 3, 2));
        let json = serde_json::to_string(&n).unwrap();
        let back: Nsga2 = serde_json::from_str(&json).unwrap();
        assert_eq!(back, n);
        let bad = json.replace("\"population\":4", "\"population\":3");
        assert!(serde_json::from_str::<Nsga2>(&bad).is_err());
    }

    #[test]
    fn telling_checks_the_goals() {
        let n = Nsga2::new(vec![unit("x")], 2)
            .unwrap()
            .with_population(4)
            .unwrap()
            .with_generations(2)
            .unwrap();
        let mut run = n.start(1);
        assert_eq!(run.candidates().len(), 4);
        assert!(
            run.candidates()
                .iter()
                .flatten()
                .all(|x| (0.0..1.0).contains(x))
        );
        let ok = vec![vec![1.0, 2.0]; 4];
        assert!(matches!(
            run.tell(&ok[..3]),
            Err(AnalysisError::Length { .. })
        ));
        let mut short = ok.clone();
        short[2] = vec![1.0];
        assert!(matches!(
            run.tell(&short),
            Err(AnalysisError::Length { .. })
        ));
        let mut nan = ok.clone();
        nan[3] = vec![1.0, f64::NAN];
        assert!(matches!(
            run.tell(&nan),
            Err(AnalysisError::Output { index: 3, .. })
        ));
        let mut minus = ok.clone();
        minus[1] = vec![f64::NEG_INFINITY, 0.0];
        assert!(matches!(
            run.tell(&minus),
            Err(AnalysisError::Output { index: 1, .. })
        ));
        let mut goals: Vec<Goals> = ok.iter().cloned().map(Goals::feasible).collect();
        goals[0].violation = -1.0;
        assert!(matches!(
            run.tell_constrained(&goals),
            Err(AnalysisError::Domain { .. })
        ));
        // A failed design may give no goals; the first generation's numbering starts at 0, the
        // second's at the population.
        goals[0] = Goals::failed();
        assert_eq!(run.tell_constrained(&goals).unwrap(), None);
        assert_eq!(run.generation(), 1);
        assert_eq!(run.members().last().unwrap().violation, f64::INFINITY);
        assert!(matches!(
            run.tell(&nan),
            Err(AnalysisError::Output { index: 7, .. })
        ));
        let front = run.tell(&ok).unwrap().unwrap();
        assert_eq!((front.evaluations, front.generations), (8, 2));
        assert!(run.candidates().is_empty());
        assert!(matches!(run.tell(&ok), Err(AnalysisError::Length { .. })));
        // The front round-trips through JSON, its infinite crowding distances as null.
        let json = serde_json::to_string(&front).unwrap();
        assert!(json.contains("null"));
        assert_eq!(serde_json::from_str::<Front>(&json).unwrap(), front);
    }

    /// Under constraints, a run that can keep them ends with a feasible front; a failed design
    /// never survives while a feasible one can take its place.
    #[test]
    fn constraints_and_failures_rank_last() {
        // Two goals x and 1 − x on [0, 1], with x ≥ 0.6, and x below 0.1 failing.
        let n = Nsga2::new(vec![unit("x")], 2)
            .unwrap()
            .with_population(20)
            .unwrap()
            .with_generations(30)
            .unwrap();
        let front = n
            .minimize_constrained(5, |x| {
                if x[0] < 0.1 {
                    Goals::failed()
                } else {
                    Goals::constrained(vec![x[0], 1.0 - x[0]], &[0.6 - x[0]])
                }
            })
            .unwrap();
        assert!(front.is_feasible());
        assert_eq!(front.members.len(), 20);
        assert!(front.members.iter().all(|m| m.point[0] >= 0.6));
        // Every design on [0.6, 1] is on the front: the front's ends reach close to both.
        let low = front.members.iter().map(|m| m.point[0]).fold(1.0, f64::min);
        let high = front.members.iter().map(|m| m.point[0]).fold(0.0, f64::max);
        assert!(low < 0.61 && high > 0.99, "{low} to {high}");
    }

    #[test]
    fn distances_by_hand() {
        let set = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
        let reference = vec![vec![0.0, 1.0], vec![3.0, 0.0]];
        // Nearest: 1 and 4 → GD 2.5; from the reference: 1 and 3 → IGD 2.
        assert_eq!(generational_distance(&set, &reference), 2.5);
        assert_eq!(inverted_generational_distance(&set, &reference), 2.0);
        assert!(generational_distance(&[], &reference).is_nan());
    }
}
