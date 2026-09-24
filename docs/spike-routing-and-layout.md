# Spike plan: WebGME connection routing and automatic layout (Rust / WASM)

**Status:** proposed  
**Goal:** Decide whether to renew autorouting and add automatic layout as WASM-backed components, and whether both can be driven by domain **profiles** (templated rulesets) rather than one-off hardcoded engines.  
**Out of scope for the spike:** shipping a production DiagramDesigner integration, or merging layout into the router.

Layout and routing stay separate products that share deployment mechanics and a profile concept.

---

## 1. Context

### Current routing

- Engine: external JS package [`webgme/autorouter`](https://github.com/webgme/autorouter) (~8k LOC).
- Integration: `ConnectionRouteManager3` + `AutoRouter.Worker.js` (already a Web Worker).
- I/O: boxes + port areas/angles + connections → orthogonal polylines.
- Command surface (via ActionApplier): `addBox`, `addPort`, `addPath`, `move`, `routeSync` / `routeAsync`, `getPathPoints`, …

### Current layout

- WebGME does not ship a first-class “arrange the boxes” engine comparable to ELK.
- Positions are largely author-owned; any auto-layout must be **opt-in**.

### Prior research already on `autorouter` `play`

- Routing alternatives: libavoid-js, PathFinding.js, JointJS manhattan, …
- Layout engines: ELKjs (ports), dagre, WebCola.

---

## 2. Questions this spike must answer

| # | Question | Success evidence |
|---|---|---|
| Q1 | Is a Rust/WASM **router** feasible behind today’s worker/command protocol? | Drop-in shim routes a recorded AR action log; tests pass on a chosen replay set. |
| Q2 | Is router performance foreseeably better than the current JS autorouter (and/or a careful TS rewrite)? | Bench numbers: median/p95 route time and memory at fixed diagram sizes; incremental move cost. |
| Q3 | Is **automatic layout** (separate crate/service) feasible for WebGME-shaped graphs (boxes, ports, hierarchy)? | Layout of representative models with fixed/free port constraints; positions applied without corrupting model semantics in a dry-run. |
| Q4 | Is layout performance foreseeably better as Rust/WASM than stock JS/TS (elkjs GWT, cola.js, …)? | Same graphs, same options: elkjs (worker) vs elkjs-wasm / elk-rs / other; wall time + memory. |
| Q5 | Can routing and layout be **profiled/templated** (ruleset in, behavior out) for domain visualizers? | Two profiles produce measurably different, valid results from one engine; invalid profiles fail loudly. |

If Q1 or Q3 fails hard, stop and document why. If Q2/Q4 show only marginal gains, prefer algorithm/profile work over a full rewrite.

---

## 3. Performance expectations (hypothesis before measuring)

### Routing

| Approach | Expected vs current JS autorouter | Notes |
|---|---|---|
| Same algorithm, ported to Rust/WASM | Modest constant-factor win (often ~1.5–3× on hot loops), less GC jitter | Worker already hides UI jank; gains show up as shorter route latency and more headroom before “feels slow.” |
| Better algorithm (visibility / 1-bend / incremental transactions) in Rust or via libavoid | Often **larger** than language alone (literature: ~10× on large orthogonal cases for 1-bend vs naïve OV graph search) | This is the real prize. |
| Naive full reroute every pointer move | Can lose to JS if marshaling dominates | Batching + dirty-net marking required. |

**Hypothesis to falsify:** At 200 boxes / ~300 nets, Rust incremental router p95 for “move one box” is ≥2× faster than JS autorouter full `routeSync`, with equal or better visual validity.

### Layout

| Approach | Expected vs JS/TS | Notes |
|---|---|---|
| elkjs (GWT) in a worker | Baseline | Mature, port-aware, already async. |
| elkjs TeaVM/WASM (upstream PR ~3× vs GWT on their suite) | Likely better than GWT JS | Still ELK semantics; lowest integration risk if it ships. |
| [elk-rs](https://github.com/openedges/elk-rs) (Rust → WASM/NAPI, elkjs-compatible API) | Potentially better than GWT; compare carefully | Maturity / parity must be validated on our models. |
| Hand-rolled Sugiyama in Rust | High effort; unlikely to beat ELK quality soon | Only if we need a tiny subset and strict MIT/control. |
| WebCola / libcola | Different problem (constraints / untangle) | Useful profile, not a layered replacement. |

**Hypothesis to falsify:** For layered ported diagrams in the 100–400 node range, a WASM/Rust ELK backend is ≥2× faster than elkjs GWT at equal layout options, without degrading a fixed parity set.

Language alone rarely beats a worse algorithm. The spike must separate **engine** wins from **algorithm** wins.

---

## 4. Template / profile concept (applies to both)

Goal: special visualizers (FSMs, dataflow, SysML-like IBDs, circuit-ish boards, …) should not fork engines. They should select or author a **profile**: a versioned ruleset that configures one shared router and one shared layouter.

### 4.1 Profile as data

```text
profiles/
  dataflow-orthogonal.v1.json
  fsm-layered.v1.json
  compact-board.v1.json
```

Suggested schema (draft for the spike to refine):

```json
{
  "id": "dataflow-orthogonal",
  "version": 1,
  "layout": {
    "engine": "elk.layered",
    "options": {
      "elk.direction": "RIGHT",
      "elk.portConstraints": "FIXED_SIDE",
      "elk.spacing.nodeNode": 40,
      "elk.edgeRouting": "ORTHOGONAL"
    },
    "hierarchy": "preserve-containers"
  },
  "routing": {
    "engine": "orthogonal.visibility",
    "grid": null,
    "costs": { "bend": 1000, "length": 1, "crossing": 500, "obstacleClearance": 8 },
    "ports": { "useAreas": true, "exitAngles": "respect" },
    "incremental": { "onMove": "dirty-nets", "duringDrag": "defer" },
    "nudging": true
  },
  "pipeline": ["layout?", "route"]
}
```

Rules the spike should prove are expressible without code changes:

- Layout direction, spacing, port constraint mode, algorithm family.
- Routing cost weights, clearance, orthogonal vs polyline, whether crossings are soft or hard.
- When to run layout vs route-only.
- Domain presets that only change numbers/enums, not Rust source.

### 4.2 What “generate a new one” means

| Level | Who authors it | Mechanism |
|---|---|---|
| **Preset** | Visualizer / meta-model | Pick a shipped profile id. |
| **Derived profile** | Power user / plugin | Clone preset, tweak options JSON, validate against schema. |
| **Custom engine plugin** (later) | Core team | New Rust strategy behind the same trait; profile selects `engine` id. |

Spike success for Q5: implement presets + derived profiles only. Custom engine plugins are a follow-on if traits are clean.

### 4.3 Risks of over-templating

- Unconstrained option soup → unreproducible layouts.
- Profiles that fight model semantics (e.g. free ports on fixed interface definitions).
- Need **validation** (JSON Schema / typestate) and golden screenshots / geometry hashes per profile.

---

## 5. Proposed repo shape (after a successful spike)

Aligns with expanding `webgme-rust-components`:

```text
Cargo.toml                          # workspace
rust-toolchain.toml
crates/
  sha1/                             # existing
  connection-router/                # boxes + ports + nets → polylines (WASM)
  auto-layout/                      # boxes + ports + nets → new box positions (WASM)
  profile-schema/                   # shared types + validation (optional thin crate)
profiles/                           # versioned JSON presets
benches/                            # criterion + node harness
fixtures/
  autorouter-replays/               # subset of AR_bug_report / test-cases JSON
  layout-models/                    # ELK JSON or WebGME-exported graphs
```

Deployment pattern matches SHA-1: worker-friendly `dist/.../web` and `dist/.../node`, contract tests that lock the JS-facing API.

---

## 6. Spike workstreams

Timebox suggestion: **2–3 weeks calendar**, two parallel tracks after a short shared scaffold.

### Track A — Connection router

**A0. Fixture & harness (2–3 days)**  
- Import 8–15 autorouter replay JSONs (basic + a few bug reports).  
- Define a minimal Rust+TS bench harness: load boxes/ports/paths → route → export points.  
- Capture JS autorouter baselines on the same machine (worker and non-worker).

**A1. Feasibility MVP (3–5 days)**  
- Rust orthogonal router MVP (grid A* **or** thin libavoid binding — pick one deliberately in day 1).  
- WASM export with a command API close to ActionApplier (`addBox`, `addPath`, `route`, `getPathPoints`).  
- Worker smoke test in Node (and a tiny browser page if cheap).

**A2. Performance & incrementality (3–4 days)**  
- Synthetic generators: N ∈ {50, 200, 500} boxes, E ≈ 1.5N nets.  
- Metrics: cold full route; move-one-box re-route; memory; wasm binary size.  
- Implement dirty-net marking; compare full vs incremental.  
- Optional: one libavoid-js run as external reference (license noted, not required for ship).

**A3. Profile hooks (1–2 days)**  
- Drive bend/length/clearance from a profile JSON.  
- Two presets (e.g. `diagram-default`, `compact-board`) produce different routes on one fixture.

**A4. Decision gate**  
- Go / no-go for replacing JS autorouter behind a feature flag.

### Track B — Automatic layout (separate)

**B0. Model bridge (2 days)**  
- Map WebGME-ish box/port/hierarchy → ELK JSON (and back to positions only).  
- Decide non-negotiables: container children, fixed port sides, rotation.

**B1. JS baseline (1–2 days)**  
- elkjs GWT in a worker on 5–10 representative graphs (include one hierarchical).  
- Record time, edge crossings proxy if available, and qualitative screenshots.

**B2. WASM/Rust candidates (3–5 days)**  
- Evaluate in order of integration risk:  
  1. elkjs WASM (TeaVM) if usable from npm/PR artifacts  
  2. elk-rs WASM API compatibility  
  3. Only if needed: WebCola for an “untangle” profile (not layered)  
- Same graphs, same options, parity checks on node order / relative layering where possible.

**B3. Profile hooks (1–2 days)**  
- Two layout profiles (e.g. `fsm-layered-down`, `dataflow-right-fixed-ports`) from one engine.  
- Validate schema; reject illegal combos (e.g. FIXED_POS without coordinates).

**B4. Decision gate**  
- Ship layout as: (a) elkjs worker wrapper only, (b) WASM ELK backend, or (c) defer.

### Track C — Shared profile system (overlaps A3/B3)

- JSON Schema for profiles; `pipeline: ["layout","route"]` dry-run.  
- Document how a visualizer domain adds a preset without touching Rust.  
- One end-to-end demo: profile → optional layout → route → SVG/HTML dump.

---

## 7. Benchmark protocol (shared)

Record for every run:

- Machine / browser or Node version / wasm feature set  
- Fixture id + profile id  
- Engine build (js autorouter / rust-grid / rust-libavoid / elkjs-gwt / elk-rs / …)  
- N, E, hierarchy depth  
- Cold time, warm time, p95 over ≥20 iterations  
- Peak JS heap / wasm memory if available  
- Validity: no node overlaps (layout), ports honored, no path through box interiors (routing), optional crossing count  

Store results under `benches/results/` as JSON so later PRs can regress.

---

## 8. Integration constraints (must not forget)

- Preserve worker message shape long enough for A/B in DiagramDesigner.  
- Port **areas**, not only points; exit angles.  
- Hierarchical boxes (parent move moves children).  
- Custom waypoints / non-auto paths must remain.  
- Layout writes positions through an explicit apply step (never surprise-move on open).  
- License: MIT-preferred for in-tree code; LGPL libavoid only as optional/benchmark or carefully reviewed dependency.

---

## 9. Decision matrix (end of spike)

| Outcome | Routing | Layout | Profiles |
|---|---|---|---|
| **Full go** | Rust/WASM router replaces JS behind flag | WASM/Rust or elkjs-wasm layout crate | Presets + derived JSON ship |
| **Partial go** | Rust router with better algorithm; keep JS fallback | elkjs GWT worker only (good enough) | Presets for routing; layout options thin wrapper |
| **No go (routing)** | Document bottleneck elsewhere (render, not route) | — | Still useful later |
| **No go (layout)** | — | Defer; authoring cost > compute gain | Keep schema design |

Minimum bar for “full go” on routing: Q1 true, Q2 ≥2× on incremental move **or** clear quality win at similar speed, worker API compatible.  
Minimum bar for layout: Q3 true, and either Q4 ≥2× **or** quality/port-handling that JS baselines cannot match for our models.

---

## 10. Deliverables

1. This plan (living doc).  
2. Spike branch with harness + fixtures (not necessarily production-ready crates).  
3. Bench report (tables + short interpretation).  
4. Profile schema draft + 2 routing + 2 layout presets.  
5. One-page recommendation: go / partial / no-go per track, with next implementation milestones.

---

## 11. Suggested first week schedule

| Day | Focus |
|---|---|
| 1 | Harness + JS autorouter baseline + pick router MVP approach |
| 2–3 | Rust/WASM route MVP + first replay fixtures green |
| 4 | Synthetic scale bench; start dirty-net design |
| 5 | ELK JSON bridge + elkjs GWT baseline |
| 6–7 | elk-rs or elkjs-wasm candidate + first profile schema |

---

## 12. Open choices to resolve on day 1 of the spike

1. Router MVP: **grid A\*** (fast to learn, MIT) vs **libavoid binding** (quality, license/weight).  
2. Layout candidate priority: **elkjs-wasm** vs **elk-rs** vs “elkjs GWT is enough.”  
3. Profile storage: repo `profiles/*.json` vs runtime registration from visualizer packages.  
4. Whether DiagramDesigner A/B needs byte-identical polylines or only “valid + stable” routes.

These are intentional spikes, not production commitments. The measurements above decide what belongs in `webgme-rust-components` next.
