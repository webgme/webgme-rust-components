# WebGME autorouter: algorithm analysis

**Source:** [`webgme/autorouter`](https://github.com/webgme/autorouter) (JS, ~8.2k LOC)  
**Purpose:** Document what the current engine *actually does*, which rules it encodes, and where algorithmic improvements are realistic for a Rust/WASM renewal.

This is a reading of the implementation (`AutoRouter.Graph`, `EdgeList`, `Path`, `Port`, `Constants`), not a restatement of the public API.

---

## 1. What kind of algorithm this is

The autorouter is **not** a classic shortest-path router (no visibility graph, no A*, no global cost minimization over bends/length/crossings).

It is a **two-phase orthogonal connector engine**:

1. **Construct** a legal right-angle polyline per connection with local geometric heuristics (buffer boxes, “hug children”, forced escapes).
2. **Relax** overlapping segments by sliding edges in separate horizontal and vertical edge lists (gap distribution / “block push”).

That lineage is closer to older interactive diagram routers (edge-list / channel compaction style) than to libavoid’s visibility-graph + A* + VPSC nudging pipeline.

| Phase | Goal | Optimality |
|---|---|---|
| Connect (`_connectPoints`) | Find *some* orthogonal path that clears inflated boxes | Greedy / heuristic; can force through boxes |
| Simplify / center stairs | Clean local geometry | Local |
| Edge-list optimize | Separate overlapping collinear segments | Iterative local; capped operations |

---

## 2. Pipeline (one full `routeSync`)

```text
for each disconnected path:
  pick start/end ports + points     (Path.calculateStartEndPorts)
  exit boxes into free space        (_getOutOfBox)
  build orthogonal polyline         (_connectPoints / hugChildren / …)
  simplify + center stairs
  register H/V edge pieces          (EdgeList.addPathEdges)

then repeatedly (_optimize, up to ~100 ops):
  simplifyPaths
  horizontal.blockScanBackward / Forward
  vertical.blockScanBackward / Forward
  horizontal/vertical.blockSwitchWrongs
```

`routeAsync` first paints temporary straight start→end segments (`_quickConnectDisconnectedPaths`), then runs the real connect + optimize in timed chunks so the UI can update.

---

## 3. Geometric model and hard rules

### 3.1 Coordinates and directions

- Continuum plane (integer-ish points), not a discrete maze grid.
- Directions: `Top=0, Right=1, Bottom=2, Left=3` (only right angles on auto routes).
- World bounds roughly `ED_MINCOORD … ED_MAXCOORD` (−2 … 100000).

### 3.2 Boxes and buffer boxes

- Every box is inflated by **`BUFFER = 10`** into a *buffer rect*.
- Touching buffer rects are **merged** into a parent “buffer box” with a list of children (`_addToBufferBoxes`).
- Routing treats these clusters as composite obstacles: paths may need to **enter the parent** and **hug children** (stay ~1px outside child edges) to escape.

**Rule:** clearance around boxes is a fixed inflate, not a per-profile cost.

### 3.3 Ports

- A port is a rectangle on a box edge (often a thin “area”), with allowed enter/exit sides (`PortStartOn*`, `PortEndOn*`).
- **Available area:** port segments occluded by sibling boxes are cut away (`adjustAvailableArea`). Attachments prefer remaining free segments.
- Attachment points avoid exact corners (ambiguous direction).
- Optional `PortConnectToCenter`.

### 3.4 Paths

- Orthogonal polylines only when auto-routed.
- Fixed / custom paths skip search and only register edges.
- Start/end direction hints (`PathStartOn*`, `PathEndOn*`) constrain port exit when valid.

### 3.5 Hierarchy

- Boxes have parent/child; children move with parents.
- Paths between ports that share a buffer parent and face each other use a **fast mid-bend** connector (`_connectPointsSharingParentBox`) instead of full search.

---

## 4. Rules implemented in detail

### 4.1 Port and endpoint selection

For each path end:

1. Prefer ports with non-empty `availableArea`; else fall back to all ports.
2. Avoid using the same port for both ends when alternatives exist.
3. Target for scoring = average of the **other** end’s port centers (or custom waypoint).
4. `getOptimalPorts` scores candidates with a hand-tuned priority:
   - Alignment with the major axis toward the target (weight ~25)
   - Minor-axis agreement (weight ~1)
   - Density factor: available length / existing attachments on that port
5. Exit direction: user hint if allowed; else `getStartEndDirTo(target)`.
6. Create the concrete point on the chosen side, snapped into the nearest available segment.

**Implication:** port choice is **local and greedy**. It does not jointly optimize (start port, end port, global route cost). Congestion is only weakly discouraged via density.

### 4.2 Leaving the box (`_getOutOfBox`)

From the port point, step in the exit direction until clear of the owner’s buffer geometry, considering the path toward the opposite endpoint.

### 4.3 Primary path construction (`_connectPoints`)

Greedy loop while `start ≠ end`:

1. Compute **major** and **minor** directions from `end − start` (`exGetMajorDir` / `exGetMinorDir`).
2. Prefer swapping major/minor if the minor matches the start hint (preserve preferred exit).
3. Advance toward the next buffer box on the major direction (`_goToNextBufferBox`).
4. If blocked / stuck, branch on geometry:
   - Straight line to end clear of children → take it.
   - End inside current buffer cluster → optionally **flip** start/end and recurse once; else `hugChildren` or **force** axis-aligned segments to the end (may cut through).
   - Obstacle with one child → walk around that child’s outer coords.
   - Obstacle with many children → `hugChildren` (wall-follow around children); on failure, jump to outer coord of the parent (again can be aggressive).

**`hugChildren` rule:** with one horizontal and one vertical direction, walk from child to child, turning clockwise/counter-clockwise, staying on the outside of children until an exit condition (leave parent, or align with endpoint, etc.). Returns `null` if it loops without exit.

**Important:** there is **no** admissible cost, **no** open set, **no** guarantee of shortest / fewest bends / fewest crossings. Failure modes are patched with flips, hugs, and forced segments—explaining many historical AR bug-report fixtures.

### 4.4 Post-connect cleanup (per path)

| Step | Rule |
|---|---|
| `_simplifyPathCurves` | Remove redundant curve-like point patterns |
| `simplifyTrivially` | Collapse collinear consecutive segments |
| `_simplifyPathPoints` | Further orthogonal simplification / 2-edge deletes when safe |
| `_centerStairsInPathPoints` | For U-shaped “stair” patterns with parallel sides, slide the middle segment to the **midpoint** between outer extents (aesthetic centering) |

Then edges are inserted into **two** global structures: `horizontal` and `vertical` `EdgeList`s.

### 4.5 Global separation / “optimization” (`EdgeList`)

After all paths are connected, the engine treats path segments as **ordered edges** on each axis and tries to keep them from stacking on top of each other.

Constants (gap model):

| Symbol | Value | Role |
|---|---|---|
| `EDLS_S` / `ED_SMALLGAP` | 15 | Desired small gap between parallel edges |
| `EDLS_R` | 16 | `SMALLGAP + 1` |
| `EDLS_D` | ~100002 | Large “graph distance” used in spacing formulas |

Mechanics (per axis):

1. Order edges; scan with a **section** of mutually overlapping (in projection) candidates.
2. `blockScanForward` / `blockScanBackward`: for each blocker, find blocked edges; **push** chains of edges using formulas `_blockGetF` / `_blockGetG` that redistribute spacing between “bracket” edges (opening/closing) and ordinary edges.
3. `_slideButNotPassEdges`: move an edge’s coordinate without illegally passing others.
4. `blockSwitchWrongs`: swap wrongly ordered pairs when heuristics say so.

`_optimize` interleaves:

1. path simplify  
2–5. H/V forward/backward scans  
6–7. H/V switch-wrongs  

Stops when a phase makes no progress, when the same phase family repeats too often (`dm` distribution limit), or when **`maxOperations` (default 100)** is exhausted.

**Rule:** parallel wires prefer ~15px separation; this is a **local sliding** process, not a global crossing-minimization ILP or VPSC solve.

### 4.6 Invalidation / incrementality (what exists today)

On box move/resize:

- Disconnect paths that **clip** the affected buffer region.
- Disconnect paths **attached** to the moved box.
- Rebuild that box’s edges; recurse to children.

Then a later `route*` reconnects disconnected paths and re-optimizes.

**Rule:** incrementality is “mark dirty by geometry intersection,” not “recompute only locally affected visibility.” A single move can invalidate many paths; reconnect still runs the full greedy connector per dirty path, and optimize still scans global edge lists (bounded by op count).

---

## 5. Explicit costs and weights (there are almost none)

Unlike libavoid / maze routers, the JS autorouter does **not** minimize a scalar such as:

`length + α·bends + β·crossings + γ·clearance_violations`

What it *does* encode as numeric preferences:

| Mechanism | Numeric rule |
|---|---|
| Box clearance | inflate by `BUFFER=10` |
| Wire–wire gap | target ~`ED_SMALLGAP=15` in edge scans |
| Port choice | major-axis weight 25, minor 1, × density |
| Optimize budget | ≤100 operations; distribution repeat cap `dm=10` |
| Async slice | default `time=5` ms between optimize ticks |

Everything else is discrete geometric casework.

---

## 6. Strengths of the current design

- Good match for **interactive** WebGME: orthogonal look, port areas, hierarchy, custom paths.
- Worker + `routeAsync` already separates UI from compute.
- Edge-list separation gives **shared-channel spacing** without a full global optimizer.
- Large replay corpus (`test/test-cases`) encodes expected behavior and regressions.
- Fast path for facing ports inside one buffer parent.

---

## 7. Limitations and algorithmic gain opportunities

Ordered from highest expected impact to more incremental cleanups.

### 7.1 Replace greedy connect with a real search (high)

**Today:** `_connectPoints` is local and can force through obstacles.  
**Gain:** orthogonal visibility graph (or 1-bend visibility) + A* with costs for length/bends/clearance—as in libavoid / Marriott–Wybrow papers.  
**Expected effect:** fewer weird detours, fewer “force to endpoint” failures, better quality at similar or better time for medium graphs; large graphs need incremental visibility updates.  
**Spike measure:** bend count, length, crossings, failure rate on AR bug JSONs vs current.

### 7.2 Joint multi-net awareness during connect (high)

**Today:** each net is routed independently; conflicts are only softened afterward by sliding.  
**Gain:** sequential routing with **edge costs that rise on occupied channels**, or rip-up-and-reroute; or global assignment of tracks in channels.  
**Expected effect:** less stacked spaghetti before optimize; optimize converges with fewer ops.

### 7.3 True incremental updates (high for interaction)

**Today:** disconnect-by-clip + full reconnect of dirty nets + global edge scans.  
**Gain:** maintain a spatial index / visibility structure; on move, update obstacles locally; reroute only incident nets; restrict edge-list scans to a y/x band.  
**Expected effect:** drag-time latency; this is where WASM helps *if* the algorithm is incremental—language alone will not fix full global rescans.

### 7.4 Principled wire spacing (medium–high)

**Today:** F/G gap formulas + iterative push with a hard op cap.  
**Gain:** VPSC / separation constraints (as in libavoid nudging) or channel compaction with explicit track assignment.  
**Expected effect:** more stable spacing, fewer leftover overlaps when `maxOperations` runs out, fewer oscillation cases (`d` / `dm` stops).

### 7.5 Better port assignment (medium)

**Today:** greedy major/minor priority independently per end.  
**Gain:** bipartite matching or beam search over (startPort × endPort × exitDirs) scored by a cheap route estimate (Manhattan + obstacles).  
**Expected effect:** fewer crossings at dense interfaces; better use of long port areas.

### 7.6 Crossings as a first-class objective (medium)

**Today:** no explicit crossing cost during connect or optimize (orthogonal crossings may remain if segments are on different axes).  
**Gain:** penalize crossings in A*; optional post-pass to uncross where topology allows.  
**Expected effect:** readability on dense diagrams; must not break intentional “must cross” cases.

### 7.7 Profile-tunable clearances (medium, product)

**Today:** `BUFFER=10`, `ED_SMALLGAP=15` are compile-time constants.  
**Gain:** expose as profile parameters (see spike plan). Even without a new algorithm, domain presets (compact board vs airy dataflow) need these knobs.

### 7.8 Smarter invalidation (medium)

**Today:** any path clipping the moved buffer rect is dropped.  
**Gain:** only paths that intersect the **symmetric difference** of old/new obstacles, or whose homotopy changes.  
**Expected effect:** fewer reconnects on tiny nudges.

### 7.9 Remove / contain “force through box” (medium, quality)

**Today:** several branches assign start onto end axes through a blocking parent when hug fails.  
**Gain:** treat as search failure → alternate exit directions, reverse path, or multi-source search from both ends (the code already flips once).  
**Expected effect:** fewer illegal overlaps; may need slightly longer routes.

### 7.10 Optimize scheduling (low–medium)

**Today:** fixed phase order + global 100-op budget can stop early with remaining overlaps.  
**Gain:** prioritize densest channels; continue until ε-stable; or separate “must-fix” vs “aesthetic” passes.  
**Expected effect:** more predictable finish quality under time budgets (`routeAsync`).

### 7.11 Data-structure / language wins (low–medium alone)

Edge lists, buffer merges, and point lists are hot. Rust/WASM will help constants and GC pauses, but **will not** fix greedy incompleteness or global O(nets × edges) scan patterns by itself.

---

## 8. Mapping to a renewal strategy

| Keep from JS autorouter | Replace early | Parameterize as profiles |
|---|---|---|
| Box / port / path domain model | `_connectPoints` search | `BUFFER`, `ED_SMALLGAP` |
| Worker + command protocol | Edge-list F/G push (optional → VPSC) | Bend/length/crossing weights |
| Custom / fixed paths | Port picker → matching | `duringDrag: defer` vs live |
| Replay fixtures as oracle | Full reconnect-on-clip | Optimize op budget / time slice |
| Hierarchy + facing-port shortcut | — | Direction / side constraints |

A pragmatic Rust MVP can keep the **same pipeline shape** (connect → simplify → separate) while swapping the connect phase for A* on an orthogonal visibility graph; that alone targets the largest quality and many performance issues. Edge-list separation can stay as v1 “good enough,” then upgrade to VPSC if benches show leftover stacking.

---

## 9. Suggested experiments (tie-in to the spike)

Use the same fixtures as `docs/spike-routing-and-layout.md`:

1. **Parity:** replay AR test-cases; classify differences as better / worse / illegal.  
2. **Stress:** N = 50 / 200 / 500; full route and single-box move.  
3. **Ablations:**  
   - greedy connect only (no optimize)  
   - greedy + edge-list (current)  
   - A* connect + edge-list  
   - A* + VPSC (or libavoid reference)  
4. **Metrics:** time, wasm/js memory, total length, bend count, segment overlaps at distance &lt; `SMALLGAP`, box interior violations.

Hypothesis: **algorithmic** ablations (3) dominate **language** ablations for quality; language + incrementality dominate interactive move latency once the connect phase is a real search.

---

## 10. One-page summary

The current WebGME autorouter implements:

- Fixed clearances (10) and wire gaps (~15)  
- Greedy orthogonal construction with buffer clusters and child-hugging  
- Local port heuristics  
- Iterative edge-list separation with a hard operation cap  
- Dirty-path reconnect on geometric clips  

It does **not** implement global shortest orthogonal routes, explicit crossing minimization, or fine-grained incremental visibility maintenance.

The largest algorithmic gains for a Rust/WASM renewal are therefore: **(1)** real orthogonal search with costs, **(2)** congestion-aware or multi-net routing, **(3)** incremental invalidation, **(4)** constraint-based spacing—while keeping the existing domain model, worker protocol, and replay tests as the compatibility layer.
