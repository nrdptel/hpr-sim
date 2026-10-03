# Ideas: avionics

Part of the [ideas backlog](ideas.md) (2026-10-03). One line each, with its tier. Several feed
[M6.4, airbrakes][m6-4], and the flight-log milestones [M7.1][m7-1] to [M7.4][m7-4].

- SOON: simulated sensor streams (barometer, accelerometer, gyro, GPS) for firmware and
  hardware-in-the-loop (HIL) tests.
- LATER: a two-way software-in-the-loop bridge to flight firmware.
- SOON (with [M6.4][m6-4]): a controller-in-the-loop Python API and a scoring harness; about 30
  student teams have built their own airbrake simulations.
- SOON: fault-injection Monte Carlo on deployment logic.
- SOON: altimeter settings export, with each device's real setting steps, and a Mach-delay
  advisor (how long an altimeter should ignore its barometer near Mach 1, where the pressure it
  reads jumps).
- SOON (with [M7.1 to M7.4][m7-1]): re-run deployment logic on a real flight log.
- SOON: sensor range checks: a barometer's ceiling (about 9,100 m, 30,000 ft, for the BMP280,
  BMP388 and BMP390) and accelerometer clipping.
- SOON: air-start tilt-lockout odds.
- SOON: staging and air-start timing, with the motor's pressure-up delay.
- SOON: predict what the altimeter will read (barometric against true altitude); contests score the
  altimeter, not the truth.
- LATER: an apogee-predictor benchmark.
- LATER: an onboard apogee predictor exported as C.
- SOON (with [M6.4][m6-4]): reference airbrake controllers: model-predictive control (MPC),
  proportional–integral–derivative (PID) and the linear-quadratic regulator (LQR).
- LATER: a pad-hold battery budget.
- LATER: an avionics data budget (sample rates against storage).
- LATER: a tracker antenna pointing table.

[m6-4]: ../decisions-and-roadmap.md#m6-4
[m7-1]: ../decisions-and-roadmap.md#m7-1
[m7-4]: ../decisions-and-roadmap.md#m7-4
