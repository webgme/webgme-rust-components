# Scientific deep dive: orthogonal connector routing and related drawing research

**Question:** Given WebGME’s current autorouter and a planned Rust/WASM renewal, should we **enhance the existing heuristic lineage**, or **diverge** toward the academic (and industrial) state of the art?

**Short answer:** For *fixed-box, ported, interactive* diagrams, the scientific mainstream since ~2009 is the **visibility-graph → A\* → order → nudge** pipeline (libavoid / Monash). WebGME’s JS autorouter is an older ad-hoc construction + edge-list sliding approach that the literature explicitly contrasts with. There has been **no paradigm-shifting breakthrough** in 2018–2025 for this exact problem; recent work mostly refines pipelines, incrementality, layered special cases, or full redraws (nodes move). Recommendation: **diverge from the current JS algorithm**, but **converge on the Monash/libavoid family** (possibly reimplemented or profiled in Rust), rather than inventing a third lineage or chasing VLSI/ML routers.

Companion docs: `docs/autorouter-algorithm-analysis.md`, `docs/spike-routing-and-layout.md`.

---

## 1. Problem taxonomy (easy to confuse)

| Problem class | Nodes | Goal | Typical methods | WebGME relevance |
|---|---|---|---|---|
| **A. Obstacle-avoiding connector routing** | Fixed | Orthogonal (or polyline) paths around boxes, often with pins/ports | Orthogonal visibility graph + A\*; 1-bend visibility; maze/A\* on grids; then ordering + nudging | **Primary** (DiagramDesigner today) |
| **B. Orthogonal graph drawing (full)** | Moveable | Embedding + bend-minimal orthogonal drawing | Topology–Shape–Metrics (TSM); Kandinsky; OGDF/yFiles | Secondary (closer to auto-layout, not pure routing) |
| **C. Layered / Sugiyama edge routing** | Layered or fixed in ranks | Orthogonal edges in channels between layers | Track assignment, pipe routing, crossing reduction in channels | Relevant for FSM/dataflow *after* layered layout |
| **D. VLSI / PCB global routing** | Pins on chips | Congestion, overflow, multi-layer, Steiner trees | Maze, ILP, differentiable / neural routers (2023–24) | Algorithm cousins only; wrong product constraints |

WebGME autorouting is **class A**. Automatic layout spikes are **B/C**. Do not judge routing success by TSM or NeuralSteiner papers without translating constraints.

---

## 2. Classic scientific stack for class A (the “forefront” that stuck)

### 2.1 Incremental polyline routing (baseline before orthogonal)

**Wybrow, Marriott, Stuckey — Incremental Connector Routing (GD 2005).**  
Maintains a visibility graph under interactive edits; routes connectors with A\*; fast enough for drag feedback for polyline avoidance. This is the architectural ancestor of “transactional” interactive routers.

### 2.2 Orthogonal connector routing (still the canonical reference)

**Wybrow, Marriott, Stuckey — Orthogonal Connector Routing (GD 2009 / LNCS 5849).**  
[PDF](https://www.csse.monash.edu.au/~mwybrow/papers/wybrow-gd-2009.pdf)

Three-phase pipeline (now industry-standard for interactive orthogonal connectors):

1. **Orthogonal visibility graph (OVG):** horizontal/vertical visibility from obstacle corners and connector ports.  
2. **A\* search:** minimize a monotonic function of **length + bend penalties** (per-connector optimality in that cost).  
3. **Visual realization:**  
   - order connectors that share a segment (avoid unnecessary crossings / bends)  
   - **nudge** with separation constraints (VPSC-style) so wires separate cleanly and prefer alley midlines when cheap  

Complexity they cite: graph construction roughly \(O(n^2)\), search roughly \(O(n^2 \log n)\) per connector in the worst case—still interactive for “reasonable” diagrams.

**Implemented in:** libavoid (Adaptagrams); used by Dunnart, Inkscape, ELK’s libavoid edge router, commercial circuit tools, etc.

This paper’s opening criticism of prior tools (“ad-hoc heuristics… routes that pass through other objects”) is essentially a description of engines in the same *family* as WebGME’s current autorouter.

### 2.3 Faster search via canonical routes

**Marriott, Stuckey, Wybrow — Seeing Around Corners: Fast Orthogonal Connector Routing (Diagrams 2014).**  
[PDF](https://www.csse.monash.edu.au/~mwybrow/papers/marriott-diagrams-2014.pdf)

- Observation: many OVG paths are **topologically equivalent**; A\* wastes time exploring them.  
- Contribution: **obstacle-hugging** canonical routes + **1-bend visibility graph** (nodes ≈ corners/endpoints; edge iff one orthogonal bend connects them).  
- Empirically: build cost can be higher, but **routing ~10× faster** on large examples vs classic OVG search in their tests.

This is the last major *algorithmic* leap specifically aimed at interactive orthogonal connector routing. libavoid evolution and forks have absorbed related ideas; full universal adoption of 1-bend as default varies by implementation.

### 2.4 Hyperedges

**Wybrow et al. — Orthogonal hyperedge routing (Diagrams 2012).**  
Extends libavoid to multi-terminal orthogonal hyperedges (circuit-like nets): construction heuristics + shared-path nudging. Relevant if WebGME domains grow hyperedges; not required for pairwise connections.

### 2.5 Separation technology underneath nudging

**Dwyer, Marriott, Wybrow — Fast node overlap removal / VPSC** (GD mid-2000s).  
Variable Placement with Separation Constraints underpins both layout non-overlap and connector nudging. Still the principled alternative to WebGME’s F/G edge-list push formulas.

---

## 3. What happened after 2014? (recent advances)

Honest assessment: **class A did not get a new dominant theory** in the last decade. Research energy split into incrementality engineering, full orthogonal drawing, layered special cases, and (separately) chip routing / learned layout.

### 3.1 Interactive / incremental refinements

**Gladisch et al. — Orthogonal Edge Routing for the EditLens (2016).**  
[arXiv:1612.05064](https://arxiv.org/abs/1612.05064)

- Needed interactive rates for lens-based graph editing.  
- Empirically: stock Wybrow/libavoid-style full recomputation became too slow (~100 ms at ~65 nodes for their scenario; seconds at ~300 nodes).  
- Response: **incremental OVG** tailored to insert/update/delete, A\* with bend/length costs, **simpler local nudging** (global VPSC nudging too expensive for their frame budget).

Takeaway for WebGME: the scientific community still builds on OVG+A\*; the live debate is **incrementality vs global quality**, not “invent a new representation.” Your spike must treat incremental dirty sets as first-class.

### 3.2 Layered / channel routers (class C)

**e.g. Method for Orthogonal Edge Routing of Directed Layered Graphs with Edge Crossings Reduction (2021).**  
Assumes nodes already on a grid of layers/lanes; routes in “pipes” with track assignment and iterative crossing repair.

Takeaway: after an ELK layered layout, a **channel router profile** may beat a free-space OVG router for that domain. That is a *profile*, not a replacement for free DiagramDesigner routing.

### 3.3 Full orthogonal drawing pipelines (class B)

**Dobler et al. — A Simple Pipeline for Orthogonal Graph Drawing (arXiv 2023).**  
[arXiv:2309.01671](https://arxiv.org/abs/2309.01671)

End-to-end: initial layout → overlap removal → edge routing → path ordering → **LP-based nudging**. Competitive experimental comparison; notes that simple routing still tends to more bends/crossings than stronger routers, with nudging as the main contribution.

**Angelini et al. — A Walk on the Wild Side: Shape-First Methodology (GD 2025).**  
Challenges classical TSM’s crossing-first priorities; argues for shape-first tradeoffs (fewer long bent edges, better uniformity). Important for **auto-layout** philosophy; less directly for fixed-box connector renewal.

Takeaway: recent GD “forefront” often assumes **you may move nodes**. WebGME routing renewal should not wait on TSM debates unless you merge with the layout track.

### 3.4 Learning-based methods (mostly other problems)

- **GNNs for graph drawing** (e.g. 2021): learn node positions / aesthetic losses—not interactive orthogonal connectors with hard obstacle avoidance.  
- **Neural / differentiable VLSI routers** (NeuralSteiner NeurIPS 2024, DGR DAC 2024, etc.): congestion and overflow on chips; GPU batch solvers; not port-area diagram editors in the browser.

Takeaway: fascinating, **not yet a practical fork** for WebGME WASM autorouting. Revisit only if you later tackle huge schematic boards with congestion metrics.

### 3.5 Engineering continuum (not papers, but “forefront” in practice)

- libavoid transaction API (`processTransaction`, connection pins, penalties: segment, crossing, port direction, buffer distance, ideal nudge, …).  
- ELK `org.eclipse.elk.alg.libavoid` exposes those as layout options—evidence the **parameterized Monash pipeline** is the interoperable standard.  
- WASM ports (libavoid-js, experimental libavoid-rust) = deployment frontier, not new math.

---

## 4. Where WebGME’s autorouter sits scientifically

| Aspect | WebGME JS autorouter | Monash / libavoid lineage | Literature status |
|---|---|---|---|
| Path existence / quality | Greedy walk + hug + force-through | Cost-optimal A\* on visibility | JS is what GD’09 criticizes |
| Obstacle model | Inflated buffer clusters | Shape polygons + buffer distance | Similar intent; different guarantees |
| Multi-net interaction | After-the-fact edge-list sliding | Shared-path ordering + VPSC nudge | JS is weaker / capped |
| Incrementality | Disconnect-by-clip, reconnect | Visibility maintenance + transactions | Both imperfect; EditLens shows still hard |
| Ports | First-class areas + angles | Connection pins / port penalties | Compatible product-wise |
| Tunability | Few constants (`BUFFER`, `SMALLGAP`) | Rich penalty vector | Profiles map cleanly to libavoid params |
| Provenance | Interactive diagram heuristics (~2010s WebGME) | Peer-reviewed + widely deployed | Clear winner as target architecture |

Conclusion: “Enhance the current algorithm” means polishing a lineage the field moved past for class A. “Diverge” should mean **adopt the scientific standard pipeline**, not invent a WebGME-only theory.

---

## 5. Diverge vs enhance — decision framing

### 5.1 Do **not** try to “just enhance” `_connectPoints` forever

Incremental patches (better hug, fewer force-throughs, higher optimize caps) hit diminishing returns: you still lack:

- admissible/consistent search costs  
- topological canonicalization (1-bend / obstacle-hugging)  
- principled multi-net ordering & nudging  

Those are structural, not constant-factor, gaps.

### 5.2 Do **diverge algorithmically** toward Monash-style class A

Recommended target architecture for Rust/WASM:

```text
shapes + pins
    → (incremental) orthogonal visibility / 1-bend graph
    → A* with profiled costs (length, bends, crossings, port dirs, clearance)
    → shared-segment ordering
    → separation / VPSC (or LP) nudging
```

Keep from WebGME: domain model (boxes, port *areas*, hierarchy, custom paths), worker protocol, replay fixtures as regression oracles.

### 5.3 How far to diverge from **libavoid itself**

| Option | Pros | Cons |
|---|---|---|
| **Wrap libavoid (C++/WASM or rust port)** | Instant SOTA behavior; ELK-compatible knobs | LGPL; large binary; harder to reshape for WebGME port areas / ActionApplier |
| **Reimplement Monash pipeline in Rust** | MIT-friendly; profile-native; learnable codebase | Multi-month; must re-validate quality |
| **Hybrid:** libavoid for spike baselines, Rust reimplementation for product | Best risk management | Two engines temporarily |

Spike recommendation: use libavoid-js (or similar) as **quality/speed oracle**, implement a **Rust OVG+A\*** MVP for product control, add 1-bend and VPSC as staged upgrades.

### 5.4 When a *further* divergence makes sense (profiles, not core)

- After layered auto-layout → optional **channel/pipe router** (2021-style).  
- Hyperedge-heavy domains → hyperedge improver.  
- Extreme interactive lenses → EditLens-style local nudge + aggressive incremental OVG.  
- Full “rearrange everything” → ELK/TSM/shape-first (**layout track**, separate crate).

These are **profile engines**, aligning with the templating idea—not reasons to skip the Monash core for free-space routing.

---

## 6. Is there a “new forefront” we would miss by converging on 2009–2014?

| Claim | Assessment |
|---|---|
| “Something after 2018 obsolete OVG+A\*” | **No** for fixed-box orthogonal connectors. |
| “1-bend visibility is unused / outdated” | Still the main published acceleration for that search; worth implementing or verifying in whatever stack you ship. |
| “LP nudging (2023) replaces VPSC” | Interesting alternative for the *nudge* phase; evaluate in spike ablations. |
| “Neural routers will win soon” | Unlikely for browser interactive ported diagrams in the near term. |
| “TSM is the real SOTA” | SOTA for *drawing the whole graph*; wrong default when authors own box positions. |

So: converging on Monash is not settling for old news; it is aligning with the **stable scientific consensus** for your problem class. Recent papers mostly tune the same skeleton.

---

## 7. Implications for the spike (actionable)

Add these literature-driven ablations to the spike benches:

1. **JS autorouter** (baseline product).  
2. **libavoid** orthogonal (scientific baseline).  
3. **Rust OVG + A\*** (no nudge / simple gap).  
4. **+ ordering + VPSC or LP nudge**.  
5. **+ 1-bend visibility** (or libavoid config if available).  
6. Optional: **layered channel router** only on ELK-laid graphs.

Metrics beyond time: length, bends, crossings, clearance violations, “through-box” count, drag increment latency.

Success criterion for “diverge”: beat JS on **illegality + bends/length**, match or beat libavoid closely enough on interactive sizes, with a profile schema that maps to segment/crossing/buffer/nudge parameters.

---

## 8. Key references (reading order)

1. Wybrow, Marriott, Stuckey — *Incremental Connector Routing* (GD 2005).  
2. Wybrow, Marriott, Stuckey — *Orthogonal Connector Routing* (GD 2009).  
3. Marriott, Stuckey, Wybrow — *Seeing Around Corners* (Diagrams 2014).  
4. Wybrow et al. — orthogonal hyperedges (Diagrams 2012).  
5. Gladisch et al. — *Orthogonal Edge Routing for the EditLens* (2016).  
6. Dobler et al. — *A Simple Pipeline for Orthogonal Graph Drawing* (2023).  
7. Handbook chapter: Tamassia et al. — planar orthogonal / polyline algorithms (class B background).  
8. Adaptagrams / libavoid docs — penalties, transactions, pins (engineering SOTA).

---

## 9. Bottom line

- **Scientific forefront for WebGME-like routing** = Monash orthogonal visibility + A\* + shared-path ordering + constraint nudging, with 2014 1-bend acceleration and ongoing incrementality engineering.  
- **Recent years** refined that stack and explored neighboring problems; they did not dethrone it for fixed-box interactive connectors.  
- **Relative to current WebGME code:** diverge algorithmically (do not eternally enhance greedy connect).  
- **Relative to the field:** do not diverge into a novel unproven core; **join the libavoid-class lineage**, implement it under Rust/WASM + profiles, and keep layout / channel / hyperedge / ML ideas as optional profile engines beside it.
