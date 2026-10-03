# Ideas backlog

Every idea from Neer's 2026-10-03 check-in, where he said "add all" to each pass, plus his own
"custom ejection charge sizing". Ideas live here, not in the roadmap
([ADR-144, the check-in's decisions][adr-144]). This page holds the rules and the scope
questions; the ideas themselves are in the theme notes below, one line each.

## Tiers

- **NEXT**: feeds a queued milestone: [M4.5, flying a `.ork` as saved][m4-5] or
  [M1.14, accuracy inside the envelope][m1-14]. Only these are in the roadmap already.
- **SOON**: the phase after the queue; most attach to an existing milestone, such as
  [M6.3, competition rules][m6-3], [M6.4, airbrakes][m6-4] or the flight-log milestones
  [M7.1][m7-1] to [M7.4][m7-4].
- **LATER**: the UI phase, community features and moonshots.
- **NEEDS NEER**: money, outreach, accounts or a change of scope. Listed below and in `STATUS.md`.
- **PROMOTED**: moved into the roadmap, with the milestone named, such as
  [M8.3, CAD interop][m8-3] from Neer's own idea of the same day.

## Promotion rule

An idea moves into the roadmap only with a named user, the workflow it serves and a *done when*.
At most 3 user-facing surfaces are in progress at once. Safety-relevant outputs follow the
guardrails in [ADR-144][adr-144]: estimates with ranges, never a go/no-go; the range safety
officer (RSO) and the safety code decide; charge sizing always says "ground test first"; each
safety number has a test pinning which way it errs.

## Theme notes

- [Pad, range and recovery hardware](ideas-pad-and-recovery.md)
- [Avionics](ideas-avionics.md)
- [Design, build, physics and trust](ideas-design-and-physics.md)
- [Flight data](ideas-flight-data.md)
- [Competitions, certification and club operations](ideas-competitions-and-clubs.md)
- [Developer ecosystem, UI and moonshots](ideas-ecosystem-and-ui.md)

## NEEDS NEER (scope, money, outreach)

- Thrust vector control (TVC) and active fins with a controller in the loop, and GPS-steered
  gliding parachutes (forum threads of about 520 and 311 replies). Both conflict with VISION's
  "steering to a target point stays out" and the airbrakes-and-canards-only rule: a scope
  decision.
- Rocket-boosted gliders: where the scope line falls.
- Bids to be the official simulator of US competitions.
- A paid team workspace (Excalidraw+ style).
- University validation partners.
- Disclosed buy links to motor.fusionspace.co, never affecting rankings.
- Verified kit badges, with kit makers.
- ThrustCurve: a data-quality give-back, and asking its maintainer about bundling common motors.
- A fiscal host or sponsor tiers (each funder's eligibility rules to be checked first).
- Asking SparkyVT's author for logs (250+ flights, to Mach 2.3).
- A migration layer for users of orhelper (a GPL-licensed Python bridge to OpenRocket): a
  licensing call.
- Prebuilt binaries through a release: a PR may prepare the workflow; creating a release is
  Neer's.
- crates.io and PyPI names, and `hpr-io`'s licence field: deferred by Neer ("not yet").

[adr-144]: ../DECISIONS.md#adr-144-the-2026-10-03-check-in-leaner-bookkeeping-issues-writing-guardrails-and-licensing-records-2026-10-03
[m1-14]: ../decisions-and-roadmap.md#m1-14
[m4-5]: ../decisions-and-roadmap.md#m4-5
[m6-3]: ../decisions-and-roadmap.md#m6-3
[m6-4]: ../decisions-and-roadmap.md#m6-4
[m7-1]: ../decisions-and-roadmap.md#m7-1
[m7-4]: ../decisions-and-roadmap.md#m7-4
[m8-3]: ../decisions-and-roadmap.md#m8-3
