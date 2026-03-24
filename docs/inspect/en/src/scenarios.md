# EdgeSentry-Inspect — Scenario Analysis

This document walks through the two deployment scenarios in depth: what happens step by step, where the hard problems are, concrete case studies, and the recommended order to implement them.

For the high-level requirements and KPIs behind these scenarios, see [requirements.md](requirements.md).
For the system architecture that supports both scenarios, see [architecture.md](architecture.md).

---

## Scenario 1: Construction Site Inspection (CONQUAS-style)

### Context

A site inspector arrives at a partially completed building with a 3D sensor device (handheld or mounted on a small rover). They need to verify that concrete work, rebar placement, wall surfaces, and structural elements conform to the approved BIM design within the specified tolerance (typically 10 mm for concrete). The entire inspection of one unit must be completed and a pass/fail verdict produced before the inspector leaves — the constraint is 30 minutes.

The inspector cannot wait for a cloud round-trip. The field PC must handle everything from scan to verdict.

### Step-by-step flow

**Step 1 — Load the design**

The inspector selects the IFC file for this unit on the field PC. `edgesentry-inspect::ifc` loads the reference geometry into a design point cloud. This happens once per session and is cached for the rest of the inspection.

**Step 2 — Scan the space**

The inspector walks the room with the 3D sensor. The sensor streams a continuous point cloud (`PointCloud`) to the field PC. `trilink-core::PoseBuffer` records the sensor pose at each capture timestamp.

**Step 3 — Project to depth map**

For each sweep, `trilink-core::project_to_depth_map` converts the 3D point cloud to a 2D depth map (`DepthMap`). Simultaneously, `trilink-core::project_to_height_map` produces a top-down `HeightMap` of the floor area. These two images are streamed to the AI inference service.

**Step 4 — AI inference (local GPU)**

The inference service runs on the field PC's GPU. It receives the depth map and height map as images and returns a `Vec<Detection>`: anomaly bounding boxes with class labels (e.g. `rebar_missing`, `surface_void`, `misalignment`) and confidence scores.

**Step 5 — Restore 3D coordinates**

For each detection, `trilink-core::unproject` maps the bounding box centre back to a world-space `Point3D` using the depth at that pixel and the pose recorded at capture time.

**Step 6 — Compute deviation**

`edgesentry-inspect::deviation` runs a k-d tree nearest-neighbour search: for every scan point, it finds the nearest design point and records the distance in millimetres. Points beyond the configured threshold (default 10 mm) are flagged.

**Step 7 — Generate heatmap and report**

`edgesentry-inspect::heatmap` projects the flagged points back to 2D with colour-coded deviation (green / yellow / red). `edgesentry-inspect::report` writes the JSON deviation report containing `compliant_pct`, `max_deviation_mm`, `mean_deviation_mm`, and the anomaly list.

**Step 8 — Inspector reviews on site**

The heatmap and report appear on the inspector's tablet. The inspector sees exactly which elements failed and by how much. The pass/fail verdict is shown before they leave the room. Total elapsed time from scan start to verdict: target under 30 minutes.

**Step 9 — Upload audit evidence**

`edgesentry-inspect::sync` uploads the report JSON and heatmap PNG to the cloud audit store (S3 Object Lock WORM). The raw point cloud is not uploaded — only the report. The upload happens in the background and does not block the on-site verdict.

### What makes this scenario difficult

| Challenge | Detail |
|---|---|
| **30-minute hard constraint** | Every processing step must run on the field PC without cloud round-trips. The projection and deviation steps must complete in seconds, not minutes. |
| **IFC geometry fidelity** | IFC files for large buildings can be complex. The `ifc.rs` loader must extract only the relevant geometry for the current unit without loading the entire building model. |
| **Partial scans** | An inspector may not scan every surface perfectly. The deviation engine must report coverage (what percentage of the design surface was actually scanned) alongside deviation. |
| **Occlusions** | Scaffolding, equipment, and workers occlude the scene. Points behind foreground objects must not be mis-attributed to the design surface behind them. The Z-buffer in `project_to_depth_map` handles this correctly. |
| **Alignment (registration)** | The scan point cloud and the IFC design cloud live in different coordinate systems until aligned. The scanner's SLAM map origin must be registered to the IFC global coordinate system before deviation can be computed. If done manually — by an operator identifying matching landmarks in the scan and the IFC model — the result depends on operator skill and introduces inconsistency between inspectors. The practical solution is fiducial markers (e.g. ArUco or AprilTag targets) placed at IFC-known coordinates before the inspection begins. The SLAM system detects the markers automatically and computes the registration without operator judgement. Manual alignment with 3 control points typically takes 5–15 minutes per unit; fiducial-assisted alignment reduces this to under 1 minute and removes operator variability. |

For detailed accuracy requirements for this scenario, see [requirements.md](requirements.md).

---

## Scenario 2: Maritime Structure Inspection

### Context

An autonomous robot (wheeled, crawling, or swimming) conducts a routine inspection of a ship hull, dock structure, or confined-space area. The robot operates independently for the duration of a mission (30 minutes to several hours). Connectivity during the mission ranges from poor to zero — the robot cannot rely on a cloud connection for any decision that needs to happen in real time. Structural changes (new corrosion, deformation, missing fasteners) must be detected and flagged during the mission so the robot can revisit a flagged area or alert the control centre immediately.

The design reference in this scenario may be a previous scan (change detection) rather than an IFC file (deviation from design). Both modes are supported.

### Step-by-step flow

**Step 1 — Load the reference model**

Before mission start, either (a) load an IFC hull design file, or (b) load a baseline point cloud from a previous inspection as the reference for change detection.

**Step 2 — Robot begins mission**

The robot navigates autonomously along a pre-planned inspection route. The 3D sensor streams a point cloud continuously. `trilink-core::PoseBuffer` records the sensor pose at each sweep timestamp.

**Step 3 — Project and infer (continuous loop, on-board)**

For each sweep, `trilink-core::project_to_depth_map` and AI inference and `trilink-core::unproject` run in sequence on the robot's on-board processor. The target latency per sweep is under 2 seconds, so the robot can slow or stop near anomalies in real time.

**Step 4 — Deviation / change detection**

Scan points are compared against the reference model using `edgesentry-inspect::deviation`. The maritime threshold is 5 mm — hull deformation tolerance is tighter than construction concrete.

**Step 5a — No anomaly: continue mission**

The robot continues on the planned route. Scan data accumulates in the local deviation log.

**Step 5b — Anomaly exceeds 2× threshold: immediate flag**

`edgesentry-inspect::sync` emits a structural-change flag to the local message queue, or directly to the control centre via radio if connectivity is available at that moment. The robot can optionally slow, stop, or re-scan the flagged area.

**Step 6 — Mission complete, robot docks**

The robot returns to its docking station on the vessel or at the facility and connects to the vessel's local network. `edgesentry-inspect::sync` uploads the full deviation report and heatmap PNG to the cloud digital twin store. The digital twin is updated with the new as-inspected geometry.

The outbound link depends on the operational context when the robot docks:

| Context | Link | Bandwidth | Feasibility for report (~1–6 MB) |
|---|---|---|---|
| Drydock or berth | Shore-side Ethernet / Wi-Fi | 10–1000 Mbps | Instant |
| At sea, within ~35 nm of shore | VDES terrestrial (VHF, ITU-R M.2092) | up to 307 kbps | ~30 sec – 3 min |
| At sea, beyond VHF range | S-VDES (satellite) or VSAT / Starlink Maritime | 100 kbps – 100 Mbps | seconds to minutes |
| Fallback | AIS messaging (legacy, pre-VDES) | ~10 kbps | marginal; JSON only, no PNG |

VDES is the IMO-standardised next-generation maritime data exchange system and the natural fit for ship-to-shore report delivery within port approaches and coastal waters. It is also the communications layer underpinning national port authority digital twin strategies, making it the recommended option for integration with those platforms. The raw point cloud is not uploaded — only the report. The 1–6 MB payload is well within VDES bandwidth even at the lower end of coastal range.

**Step 7 — Control centre review**

Engineers review the uploaded report. Flagged structural changes are prioritised for maintenance. The updated digital twin shows the current state of the asset.

### What makes this scenario difficult

| Challenge | Detail |
|---|---|
| **Zero connectivity during mission** | No cloud calls are possible in confined spaces or underwater. Every decision must be made locally. The robot must buffer the full deviation log and sync it after docking. |
| **AI model must run on robot hardware** | The inference service runs on the robot's on-board SoC (NVIDIA Jetson, similar). The model must be quantised to fit memory and compute constraints without degrading accuracy below the 5 mm detection threshold. This quantisation is an ongoing operational burden — every model update requires re-quantisation and re-validation. |
| **Change detection vs. design deviation** | For vessels where the as-built state already deviates from the original design (common in older ships), using the IFC design file as the reference produces false positives. Instead, a previously accepted baseline scan is used as the reference. The system must support both modes. |
| **Pose accuracy in GPS-denied environments** | SLAM accuracy degrades in featureless confined spaces (smooth hull plates, flooded bilge tanks). Pose drift accumulates over a long mission and degrades the 5 mm position accuracy requirement. Loop closure or fiducial markers must be used at regular intervals. |
| **Variable lighting and surface conditions** | Corrosion, marine growth, and water accumulation on hull surfaces affect the point cloud density and AI inference quality differently than a clean construction site. |

For detailed accuracy requirements for this scenario, see [requirements.md](requirements.md).

---

## Deployment Comparison

| Aspect | Scenario 1 (Construction) | Scenario 2 (Maritime) |
|---|---|---|
| **Connectivity** | Available (field PC on Wi-Fi or LTE) | Not available during mission |
| **Reference model** | IFC design file | IFC or previous scan (change detection) |
| **Deviation threshold** | 10 mm | 5 mm |
| **Time constraint** | 30 min (hard) | Per-mission (hours); verdict not needed on site |
| **Inference hardware** | Field PC GPU (no quantisation) | Robot SoC (quantisation required) |
| **On-site feedback** | Inspector tablet / AR headset | Robot slows / stops at anomaly; flag to control centre |
| **Cloud sync trigger** | After verdict, in background | After docking |
| **Model update cadence** | Update inference endpoint only | Re-quantise + re-validate + redeploy to robot |
| **Alignment complexity** | SLAM → IFC registration; fiducial markers strongly recommended to remove operator variability | Same, plus SLAM drift management over long missions |
| **EdgeSentry-Inspect code change** | None | None — `inference.base_url` points to `localhost`; threshold configured |
| **Overall difficulty** | Medium | High |

The EdgeSentry-Inspect codebase is identical for both scenarios. The difficulty difference is almost entirely in the inference hardware layer (quantisation for Scenario 2) and the connectivity layer (offline buffering for Scenario 2).

---

## Case Studies

### Case Study A — High-rise apartment handover inspection (Scenario 1)

**Operator:** A main contractor delivering a 40-storey residential tower.

**Environment:** Indoor apartment units, typically 60–90 m² each. 320 units total. Elevator access. 240V power available throughout.

**Problem:** Before handover to the developer, every unit must pass a structural inspection: wall flatness, floor levelness, ceiling height, opening dimensions. Currently, a three-person team spends 45–60 minutes per unit using spirit levels and tape measures. At 320 units the total inspection takes 6–8 weeks. Disputes about marginal non-conformances are common.

**Deployment:**

- Inspector brings a field PC and a handheld 3D sensor into each unit.
- IFC model for the floor plate is pre-loaded on the field PC (one file covers all units on that level with offsets).
- Inspector scans the unit in a single walk-through (approximately 15 minutes).
- EdgeSentry-Inspect produces a deviation report showing all elements outside the 10 mm tolerance within 5 minutes of scan completion.
- Total time per unit: ~20 minutes including setup.

**Outcome:**

- Inspection time reduced from 45–60 minutes to 20 minutes per unit (55% reduction).
- Non-conformances are documented with millimetre-level precision and photographic evidence — disputes are resolved by the data, not by argument.
- Report is uploaded to the project's common data environment and linked to the IFC model automatically.

**Why this scenario is straightforward:**
Stable indoor environment, good sensor range, no connectivity constraint. The field PC has a standard GPU. No model quantisation required.

---

### Case Study B — Public infrastructure: MRT station concourse (Scenario 1)

**Operator:** A civil engineering contractor completing a new metro station.

**Environment:** Large open concourse, ~2,000 m² floor area, ceiling height 8–12 m. Construction is ongoing in adjacent areas. Equipment and workers present during inspection windows.

**Problem:** Inspection windows are narrow (2–4 hours at night) due to ongoing construction. A full flatness and alignment survey of all structural elements must be completed within the window. Traditional total-station survey takes 6–8 hours for a space this size. The station cannot open without a signed-off deviation report for each structural element.

**Deployment:**

- A rover-mounted 3D sensor is driven through the concourse by a single operator.
- The 8–12 m ceiling height requires a sensor with longer range than a handheld device (trade-off: lower density at distance).
- EdgeSentry-Inspect adjusts the deviation threshold dynamically by element type: 5 mm for column faces, 15 mm for wall panels at ceiling height.
- The full concourse scan is completed in 90 minutes; the deviation report is ready 10 minutes after the scan ends.

**Key complexity vs. Case Study A:**

- Dynamic threshold by element type (not a single global threshold).
- Large scan area requires stitching multiple sweeps (the SLAM system handles this; EdgeSentry-Inspect receives a unified point cloud).
- Worker and equipment occlusions are higher — coverage reporting is critical to flag under-inspected areas.

---

### Case Study C — Drydock hull inspection (Scenario 2)

**Operator:** A ship repair yard conducting a class renewal survey for a 180-metre bulk carrier.

**Environment:** Ship in drydock. Hull is accessible from ground level and via scaffolding. Some confined ballast tank spaces require a crawling robot. No cellular coverage inside the tanks.

**Problem:** A class renewal survey requires documenting the thickness and surface condition of the entire hull. Traditional ultrasonic thickness gauging and visual inspection requires 30–40 surveyors working for 3–5 days. The yard wants to reduce survey time to 1 day and produce a digital record that can be compared against the vessel's previous survey.

**Deployment (external hull, drydock):**

- A wheeled robot with a 3D sensor and on-board GPU crawls the external hull surface.
- Reference model: previous survey scan (3 years ago), not the original IFC (the vessel has been modified since build).
- EdgeSentry-Inspect runs in change-detection mode: new scan vs. previous scan.
- Deviation > 5 mm (interpreted as surface wastage or deformation) triggers an immediate flag to the yard control room via radio (connectivity available for external hull in drydock).
- Robot completes the external hull in 8 hours. Report uploaded at mission end.

**Deployment (confined ballast tanks):**

- A smaller crawling robot enters the tank through the access manhole.
- No connectivity inside the tank.
- Deviation flags accumulate in the local buffer.
- When the robot exits the tank, deviation log is synced automatically.
- Control room reviews all tank reports after the tank inspection session.

**Outcome:**

- Survey time reduced from 3–5 days to approximately 28 hours (hull + tanks).
- Digital deviation map is directly comparable against the previous survey — structural change over the 3-year period is immediately visible as a colour-coded overlay.
- Classification society accepts the digital report as primary evidence (paper sketches are no longer required).

**Key complexity vs. Case Study A:**

- Two sub-scenarios in one deployment: connected external hull + disconnected confined tanks.
- Quantisation required for the crawling robot SoC.
- Change detection mode instead of IFC deviation mode.
- Sync-after-docking logic must handle partial reports gracefully (robot may need to exit and re-enter a tank multiple times).

---

## Recommended Implementation Order

**Implement Scenario 1 (construction, connected) first.**

### Rationale

1. **The 30-minute constraint validates the entire edge pipeline.**
   If the full cycle — project → infer → unproject → deviation → report — can run within 30 minutes on a field PC, Scenario 2 (which has no hard on-site time constraint) is straightforwardly achievable with the same code.

2. **No quantisation dependency.**
   Scenario 1 runs on a standard field PC GPU. There is no dependency on a robot hardware team or model quantisation toolchain. The pipeline can be built, tested, and demonstrated without hardware partners.

3. **IFC deviation mode is the foundation for change-detection mode.**
   Scenario 2's change-detection mode (new scan vs. previous scan) reuses the entire deviation engine — the "design reference cloud" is simply replaced by a previous scan cloud. Implementing IFC deviation first means Scenario 2 requires no structural code change.

4. **A working Scenario 1 deployment is the proof of value needed to justify Scenario 2 investment.**
   Convincing a ship repair yard or port authority to trial an autonomous robot requires evidence that the AI + deviation pipeline produces reliable results. A construction site handover inspection (lower operational complexity, easier access, controlled environment) is the right first deployment to generate that evidence.

5. **Scenario 2 adds dependencies outside EdgeSentry-Inspect's control.**
   Robot SoC quantisation, SLAM accuracy in GPS-denied environments, and mission planning are all provided by the robot platform partner. Those integrations are easier to negotiate and execute after a live Scenario 1 deployment has demonstrated the pipeline's accuracy.

### Suggested phasing

| Phase | Scenario | Target use case | Prerequisite |
|---|---|---|---|
| **Phase 1** | Construction site inspection | Apartment handover, civil infrastructure | trilink-core #30–#34 merged; M2–M4 complete |
| **Phase 2** | Maritime — external (connected) | Drydock hull survey, dock structure | Phase 1 reference deployment; at least one confirmed customer |
| **Phase 3** | Maritime — confined (offline robot) | Ballast tanks, engine rooms, underwater | Phase 2 complete; robot partner confirms quantisation and offline sync |

Phase 3 requires no changes to EdgeSentry-Inspect code. The investment is entirely in the robot platform integration layer (quantisation, fleet management, sync-after-docking retry logic) — work that is justified by Phase 2 results.

For a breakdown of the factors that determine measurement accuracy in the field, see [architecture.md](architecture.md).
