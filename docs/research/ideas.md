# Ideas backlog

Every idea from Neer's 2026-10-03 check-in, where he said "add all" to each pass, plus his own
"custom ejection charge sizing". Ideas live here, not in the roadmap (ADR-144). This page holds the
rules and the scope questions; the ideas themselves are in the theme notes below, one line each.

## Tiers

- **NEXT**: feeds a queued milestone (M4.5, M1.14). Only these are in the roadmap already.
- **SOON**: the phase after the queue; most attach to an existing milestone (M6.3, M6.4, M7.x).
- **LATER**: the UI phase, community features and moonshots.
- **NEEDS NEER**: money, outreach, accounts or a change of scope. Listed below and in `STATUS.md`.

## Promotion rule

An idea moves into the roadmap only with a named user, the workflow it serves and a *done when*.
At most 3 user-facing surfaces are in progress at once. Safety-relevant outputs follow the
guardrails in ADR-144: estimates with ranges, never a go/no-go; the range safety officer and the
safety code decide; charge sizing always says "ground test first"; each safety number has a test
pinning which way it errs.

## Theme notes

- [Pad, range and recovery hardware](ideas-pad-and-recovery.md)
- [Avionics](ideas-avionics.md)
- [Design, build, physics and trust](ideas-design-and-physics.md)
- [Flight data](ideas-flight-data.md)
- [Competitions, certification and club operations](ideas-competitions-and-clubs.md)
- [Developer ecosystem, UI and moonshots](ideas-ecosystem-and-ui.md)

## NEEDS NEER (scope, money, outreach)

- Thrust vector control and active fins with a controller in the loop, and GPS-steered gliding
  parachutes (forum threads of about 520 and 311 replies). Both conflict with VISION's "steering
  to a target point stays out" and the airbrakes-and-canards-only rule: a scope decision.
- Rocket-boosted gliders: where the scope line falls.
- Bids to be the official simulator of US competitions.
- A paid team workspace (Excalidraw+ style).
- University validation partners.
- Disclosed buy links to motor.fusionspace.co, never affecting rankings.
- Verified kit badges, with kit makers.
- ThrustCurve: a data-quality give-back, and asking its maintainer about bundling common motors.
- A fiscal host or sponsor tiers (each funder's eligibility rules to be checked first).
- Asking SparkyVT's author for logs (250+ flights, to Mach 2.3).
- An orhelper migration layer: a GPL licensing call.
- Prebuilt binaries through a release: a PR may prepare the workflow; creating a release is
  Neer's.
- crates.io and PyPI names, and `hpr-io`'s licence field: deferred by Neer ("not yet").
