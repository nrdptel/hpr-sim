# Ideas: avionics

Part of the [ideas backlog](ideas.md) (2026-10-03). One line each, with its tier. Several feed the
airbrakes milestone (M6.4) and the flight-log milestones (M7.x).

- SOON: simulated sensor streams (barometer, accelerometer, gyro, GPS) for firmware and
  hardware-in-the-loop tests.
- LATER: a two-way software-in-the-loop bridge to flight firmware.
- SOON (with M6.4): a controller-in-the-loop Python API and a scoring harness; about 30 student
  teams have built their own airbrake simulations.
- SOON: fault-injection Monte Carlo on deployment logic.
- SOON: altimeter settings export, with each device's real setting steps, and a Mach-delay advisor.
- SOON (with M7): re-run deployment logic on a real flight log.
- SOON: sensor range checks: a barometer's ceiling (about 30,000 ft for the BMP280, BMP388 and
  BMP390) and accelerometer clipping.
- SOON: air-start tilt-lockout odds.
- SOON: staging and air-start timing, with the motor's pressure-up delay.
- SOON: predict what the altimeter will read (barometric against true altitude); contests score the
  altimeter, not the truth.
- LATER: an apogee-predictor benchmark.
- LATER: an onboard apogee predictor exported as C.
- SOON (with M6.4): reference airbrake controllers (MPC, PID, LQR).
- LATER: a pad-hold battery budget.
- LATER: an avionics data budget (sample rates against storage).
- LATER: a tracker antenna pointing table.
