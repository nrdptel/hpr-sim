# Time integration and events

Code: `hpr_sim::integrator` and `hpr_sim::events`. Decisions: ADR-010.

Sources:

- **[DP80]** J. R. Dormand and P. J. Prince, "A family of embedded Runge-Kutta formulae",
  *J. Comput. Appl. Math.* 6 (1980) 19–26, doi:10.1016/0771-050X(80)90013-3. Cited, not fetched
  (the publisher refuses scripted downloads).
- **[HNW]** E. Hairer, S. P. Nørsett and G. Wanner, *Solving Ordinary Differential Equations I*,
  2nd ed., Springer, 1993: §II.1 (RK4, table 1.2), §II.2 (order conditions, table 2.2), §II.4
  (error norm, step control, starting step), §II.5 (DOPRI5, table 5.2), §II.6 (dense output).
  The PI controller is §IV.2 of volume II. Cited, not fetched.
- **[D5]** E. Hairer and G. Wanner, `DOPRI5` (Fortran, version of 2004), BSD-2-Clause, pinned as
  `hairer-dopri5` in `validation/refs.lock.toml`. It is the book's code, and the port follows it:
  coefficients (`CDOPRI`), the error norm and controller (`DOPCOR`), the starting step (`HINIT`) and
  the dense output (`CONTD5`).
- **[B73]** R. P. Brent, *Algorithms for Minimization without Derivatives*, Prentice-Hall, 1973,
  ch. 4 (the zero finder). Cited, not fetched; implemented from the algorithm's description.

## Methods

- **Dormand–Prince 5(4)** ([DP80]; [HNW] table 5.2). Seven stages, first-same-as-last, so six new
  evaluations per step. The step advances with the fifth-order solution (local extrapolation), and
  the embedded fourth-order solution estimates the error: `err_i = h Σ (b_j − b̂_j) k_j`.
- **Error norm** ([HNW] eq. 4.11; [D5]): `‖err‖ = √(Σ (err_i/sc_i)²/N)` with
  `sc_i = w_i·atol + rtol·max(|y0_i|, |y1_i|)`. The weights `w_i` come from
  `OdeSystem::absolute_tolerance_weights`, so components in different units share one `atol`.
  A step is accepted when `‖err‖ ≤ 1`.
- **Step control** ([HNW] II, §IV.2; [D5] defaults). The PI controller
  `h_new = h / clamp(‖err‖^(0.2 − 0.75β) / ‖err_old‖^β / 0.9, 1/10, 5)`, with `β = 0.04`.
  After a rejection, `h_new = h / min(5, ‖err‖^0.17/0.9)`, and the step after a rejection can't grow.
- **Departures from `DOPRI5`.**
  - A non-finite error estimate, or a derivative that fails in stages 2–7, counts as a rejection
    that shrinks `h` five times. A long step's stages can probe states off the trajectory
    (negative mass, a Mach number past a table's end). The failure is reported only once the step
    can't shrink further. A failure at the step's first stage is reported at once, and so is any
    failure under RK4.
  - A final interval within the rounding of `t` (`0.1 h ≤ max(|t|, 1)·uround`) counts as
    reached, instead of failing as too small.
  - After a step shortened to land on a stop time, the next call starts from the larger of the
    controller's proposal and the step the controller wanted before shortening.
  - The step size overflowing to infinity (a problem with zero error and no stop) is an error.
  - There is no stiffness detection.
- **Starting step** ([HNW] §II.4; `HINIT`): `h₀ = 0.01 ‖y₀‖/‖f₀‖`, then one Euler step estimates
  the second derivative and `h = min(100 h₀, (0.01/max(‖f₀‖, ‖y''‖))^(1/5))`.
- **Dense output** ([HNW] §II.6; `CONTD5`). With `θ = (t − t₀)/h` and `Δ = y₁ − y₀`:
  `y(θ) = y₀ + θ(Δ + (1−θ)(r₂ + θ(r₃ + (1−θ) r₄)))`, where `r₂ = h k₁ − Δ`, `r₃ = Δ − h k₇ − r₂`
  and `r₄ = h Σ d_j k_j`. It is a continuous extension of order 4, so its error inside a step is
  `O(h⁵)`.
- **RK4** ([HNW] table 1.2): weights 1/6, 2/6, 2/6, 1/6 at 0, ½, ½, 1, with a fixed step. Its
  dense output is the cubic Hermite interpolant of `y₀, y₁, f₀, f₁` in the same nested form without
  `r₄` (error `O(h⁴)` inside a step). `f₁` is the next step's `k₁`, so it costs nothing extra.

## Stop times and discontinuities

- `Integrator::advance(system, t_stop)` never steps past `t_stop`. Like `DOPRI5`, it stretches the
  last step up to 1% to land on it exactly, and `time_s()` equals `t_stop` bit for bit.
- The system is one `OdeSystem<N>`: the derivative, optional tolerance weights, optional events
  (`event_count`, `event_direction`, `event_value`), and `accept_step`. `accept_step` sees every
  accepted step with its dense output and can stop the run (`Advance::Stopped`). So a flight phase
  can record from, and stop on, its own state.
- A discontinuity in the right-hand side (burnout, a staging, a phase change) must be a stop time.
  An RK step across a jump drops to first order (Loft lesson L23).
- **Which side a stop time belongs to.** The last stage of the step ending at `t_stop` is evaluated
  *at* `t_stop` and belongs to the phase before it. The first stage of the next call belongs to the
  phase after it. A system that picks its phase from `t` alone gets one of the two wrong, so the
  caller sets the phase between calls (`integrator::tests::a_discontinuity_inside_a_step_...`).
- Each call evaluates `f(t, y)` afresh, so the system can change between calls. The step-size
  estimate carries over.

## Events

- An event is a scalar `g(t, y)` with a direction: rising (`g0 < 0 ≤ g1`), falling (`g0 > 0 ≥ g1`)
  or either. The start value must be strictly on one side.
- After each accepted step, `advance` evaluates every `g` at the step's end. For each sign change it
  finds the zero of `g(t, y_dense(t))` on the step with Brent's method ([B73]) to
  `EVENT_TIME_RESOLUTION_S = 1e-12 s` (plus `4ε|t|`). The earliest zero sets the stop.
- **Where it stops.** The integrator stops at the end of Brent's final bracket on the far side of
  the zero, with the dense output's state there. It returns `Advance::Events`, and
  `fired_events()` lists every event past its zero at that state, ascending. Coincident events,
  such as two devices set to deploy at apogee, fire together. `g` is already past zero (or
  exactly zero) for all of them when the next call starts, so none is reported twice. The system's
  `accept_step` sees the shortened step.
- **Accuracy.** The event time is as accurate as the solution: the root finder adds at most about
  `2e-12 s`. The state at the event is the dense output's, which is fourth order for
  Dormand–Prince and third for RK4.
- **Limits.**
  - A function that crosses zero and returns inside one step goes unseen. Bound the step with
    `Adaptive::max_step_s` where that can happen.
  - A function that crosses and comes back to exactly zero at the step's end is reported at the
    end.
  - Events are checked only on accepted steps, never on stage values.
  - `Integrator::reset` between calls can move an event function back to its near side, by as
    little as renormalizing a quaternion. The event then fires again moments later. So normalize
    inside the system's functions rather than resetting at an event.

## Defaults and limits

- `rtol = atol = 1e-8`, no maximum step, and a limit of 10⁶ attempted steps.
- **M1.6b decides the flight's weights and tolerances** from its benchmark and accuracy.
- A stiff flight phase shows up as `StepTooSmall` or the step limit.
- Integration runs forward only. `t_stop` may be infinite, to run until an event or a stop from
  `accept_step`. After any error, the integrator stays at its last accepted step and can resume;
  `set_step_limit` raises a spent limit.

## Verification

Unit tests in `hpr_sim::integrator::tests` and `hpr_sim::events::tests`. The numbers quoted were
measured on 2026-09-17.

- **Tableau.** Rows sum to `c`. The fifth-order weights satisfy all 17 order conditions to
  order 5 ([HNW] table 2.2) to 1e-14. The embedded weights satisfy orders 1–4 and miss at least one
  fifth-order condition by more than 1e-4. The dense output's weights `b(θ)` satisfy the conditions
  to order 4 at θ = 0.1 to 0.9, which pins `D1`–`D7`.
- **The port is `DOPRI5`.** On Hairer's driver problem, the Arenstorf orbit, at `rtol = atol` = 1e-4,
  1e-7 and 1e-10, the evaluations (494, 1442, 5060), attempted steps (82, 240, 843) and accepted
  steps (64, 216, 841) equal those of an independent line-by-line transcription of `DOPCOR` and
  `HINIT`. So do the end positions, to 1e-9. This pins the controller, the starting step and the
  error norm.
- **Step-halving convergence** on the nonlinear, non-autonomous `y' = −2ty²`, `y = 1/(1 + t²)` on
  `[0, 2]`, driven through `advance`:
  - Dormand–Prince at a fixed step (first and largest step `h`, tolerances too loose to reject),
    10 to 160 steps: orders 5.88, 5.53, 5.31, 5.17, falling toward 5. At 320 steps the error, 6e-15,
    reaches rounding. The dense output at `θ` = ¼, ½, ¾ gives 4.81, 4.97, 4.99, 4.99.
  - RK4, 20 to 160 steps: orders 4.04, 4.02, 4.01. Its Hermite dense output gives 3.96, 3.99, 4.00.
- **Tolerance proportionality.** Over `[0, 10]` with `rtol = atol` from 1e-4 to 1e-9, the error
  falls from 2.7e-5 to 2.2e-10, 0.22 to 0.28 of the tolerance throughout (asserted within 0.07 to
  0.85).
- **Apogee under tolerance halving (L21).** A vertical flight with quadratic drag, `v' = −g − kv|v|`,
  has a closed-form apogee (`testing::closed_form_quadratic_drag`). As the tolerance halves from
  1e-5 to 2e-8, the apogee time error falls from 5.7e-4 s to 1.2e-8 s and the height error from
  3.4e-4 m to 8.3e-8 m. The fall is not monotone step by step: `v|v|` has a kink at apogee. The
  log–log slope of the height error against the tolerance is 1.36, and the worst errors are 117·tol
  (time) and 91·tol (height).
- **Vacuum with constant thrust (L23).** Burnout is a stop time. At burnout the state matches
  Tsiolkovsky's equation with gravity loss to 1e-9 relative, for both methods. The coast apogee is
  within 6.3e-9 s and 5.2e-6 m of 48.8 km (Dormand–Prince, 20 steps) and 4e-11 s and 1.6e-8 m (RK4,
  0.01 s). Integrated straight through the jump, RK4 misses the velocity by 0.56 m/s; with the stop
  time the miss is 1e-12 m/s.
- **Events within 1e-6 s (L22).**
  - For the quadratic-drag flight, the apogee, a 300 m descending deploy and landing are all
    located against the closed forms. Errors: 1.2e-8, 1.5e-8 and 1.4e-8 s at the default
    tolerances, and 3e-12 to 1.3e-11 s with RK4 at 0.01 s. `g` at the stop is below 1e-13.
  - For `x = cos t`, falling zeros and extrema over 20 s are located to 3e-8 s (Dormand–Prince) and
    1.6e-9 s (RK4).
  - Restarting on an event doesn't report it again.
  - Three coincident events fire together, once per crossing.
  - Three crossings inside one RK4 step come out in time order, to 1e-12 s.
  - Where the dense output is exact (`y' = 1`), nonlinear event functions stop between 0 and
    2.5e-12 s past their roots, on the far side.
- **Brent.** Checked on a smooth root to 1e-14, a triple root, a step discontinuity (under 60
  evaluations, far side returned), exact zeros at the ends, a bracket that doesn't bracket and a
  NaN.
- **Failures.** These are reported as errors, never as a quiet stop:
  - a failing derivative (the adaptive method closes to within 1e-9 s of where it fails);
  - a blow-up (`y' = y²` stops near `t = 1`);
  - a step overflowing to infinity;
  - the step limit (and resuming after raising it);
  - bad settings, a backward stop and zero weights.
- **Robustness.** Stops 1 to 40 ulps ahead of `t` = 0, 3, 100 and 1000 s are reached. Steps respect
  `max_step_s`. `accept_step` can stop a run. Repeated runs are bit-identical. Unknown fields and
  the old tag names are refused in settings files.
