# Ideas: developer ecosystem, UI and moonshots

Part of the [ideas backlog](ideas.md) (2026-10-03). One line each, with its tier. AI features
never write numbers or make safety calls: every number comes from the engine.

## Developer ecosystem

- SOON: an MCP server (Model Context Protocol, the standard way for AI assistants to call tools).
- SOON: notebook widgets.
- SOON: a GitHub Action for design CI.
- LATER: a sandboxed WebAssembly (WASM) plugin API, with Rhai (a small embedded scripting
  language for Rust) for light scripting.
- SOON: the JSON Schema on SchemaStore.
- LATER: an editor extension and language server.
- LATER: MCAP export (an open log format for robotics data, read by ROS 2 and Foxglove).
- LATER: oEmbed embeds.
- LATER: FMU export (a Functional Mock-up Unit, the FMI standard's packaged co-simulation model).
- LATER: CZML (Cesium's format for time-dynamic 3D scenes) and animated glTF.
- SOON: Arrow and Parquet batch output.
- LATER: Grafana overlays.
- LATER: Pyodide and JupyterLite (Python in the browser).
- LATER: a Discord bot.
- LATER: Excel functions.
- SOON: a simulator-neutral conformance suite.
- LATER: an OBS (streaming software) overlay.
- SOON: import on-site wind soundings.
- LATER: the MCP server as an AI design benchmark (RocketBench, [arXiv 2504.19394](https://arxiv.org/abs/2504.19394)).

## UI and experience (the UI phase)

- NEXT: plots and figures first ([M4.5e, `hpr sim --plot`][m4-5e];
  [M1.14g, figures on the accuracy pages][m1-14g]).
- LATER: grounded number chips: each number links to where the engine computed it.
- LATER: one time cursor across 3D, plots and video (Rerun-style timelines).
- LATER: a scrubbable edit-history sparkline.
- LATER: a configurations grid (Onshape style).
- LATER: auto-director replay clips.
- LATER: a command palette with a typed grammar.
- LATER: plain language to a competition spec to the optimizer.
- LATER: a critique with previewable fixes.
- LATER: ask-the-flight, answers citing time ranges.
- LATER: a local large language model (LLM) option, running on the user's own machine.
- LATER: linked user part components (Figma style).
- LATER: pencil sketching (Shapr3D style).
- LATER: shareable panel layouts.
- LATER: Strava-style flight cards.
- LATER: photoreal launch sites.
- LATER: live force arrows in the replay.
- LATER: stability modes against flight time.
- LATER: shareable flight links.
- LATER: local-first designs: CRDTs (conflict-free replicated data types, which merge edits made
  offline), branches, comments.
- LATER: a classroom mode and a data pack for NGSS (the US Next Generation Science Standards).
- LATER: leaderboards, closest-to-prediction included.
- LATER: accessibility: screen-reader plots and sonification.
- LATER: country regulation profiles (UK, Canada) and translations, French first.
- LATER: "your flight against others on this kit and motor".

## Moonshots (LATER)

- LATER: an open drag commons (opt-in, privacy-safe).
- LATER: a blind flight index: predictions committed before launch, scored after.
- LATER: `hpr-core` as `no_std` on flight computers; the first step is an embedded build check.
- LATER: `.hpr` handed to neutral governance (as glTF was).
- LATER: signed flight passports.
- LATER: vortex-lattice aerodynamics in the browser (WebGPU).
- LATER: pad-side trajectory capture.
- LATER: motor scatter from certification data.
- LATER: shared range winds.
- LATER: sanctioned virtual events.
- LATER: a curriculum partner, in the style of PhET (the University of Colorado's free
  interactive science simulations).

[m1-14g]: ../decisions-and-roadmap.md#m1-14g
[m4-5e]: ../decisions-and-roadmap.md#m4-5e
