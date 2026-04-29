# Why EdgeSentry-Inspect?

This document explains the problem EdgeSentry-Inspect addresses, the pain points of current inspection practice, and how it differs from existing solutions on the market.

---

## The problem

Infrastructure inspection — whether on a construction site or a ship hull — is one of the last major engineering workflows that still relies heavily on manual measurement: a person with a spirit level, a tape measure, and a clipboard.

For construction handover inspections, a three-person team typically spends 45–60 minutes per residential unit verifying wall flatness, floor levelness, ceiling height, and opening dimensions. For a 320-unit building, that is 6–8 weeks of inspection time. Disputes about marginal non-conformances are common, because measurements are taken by hand and are not spatially repeatable.

For maritime hull surveys, 30–40 surveyors work for 3–5 days to cover a single vessel. Results are recorded on paper sketches. There is no digital record that can be compared against the survey from three years ago. Every classification renewal starts from scratch.

Neither workflow can meet the demands of modern regulatory programmes, which increasingly require automated, auditable, and spatially precise inspection records.

---

## Pain points

**Speed:** Manual measurement cannot meet the 30-minute inspection window required for automated regulatory compliance in construction. A 4-hour autonomous robot hull survey is impossible without a fully offline pipeline.

**Precision:** Human measurements with a tape measure and spirit level carry ±5–10 mm variability. For structural elements where the tolerance is 10 mm, that variability is the entire tolerance budget. Results are not repeatable between inspectors.

**Cost:** Large teams, long timelines, and repeated work to resolve disputes all accumulate cost. The majority of that cost is labour — not capital equipment — which makes it a fixed operational expense that does not scale down.

**No spatial context:** A manual report says "Column C4 is 12 mm out of tolerance." It does not say which face, at what height, over what area. Without spatial context, the contractor cannot confirm the finding or plan a targeted remediation.

**No historical comparison:** For maritime assets, there is no automated way to compare the current inspection against the one from the previous survey cycle. Structural degradation trends are invisible until a failure occurs.

**Connectivity constraints:** Construction sites and vessels often have limited or no internet connectivity during the inspection window. Cloud-only platforms cannot return a verdict until the data has been uploaded and processed remotely — which may take hours or be impossible entirely.

---

## Existing solutions and their gaps

| Category | Examples | Gap |
|---|---|---|
| General 3D scanning software | Faro Scene, Leica Cyclone | No AI anomaly detection; no BIM deviation comparison; results require offline post-processing on a workstation; no edge pipeline for real-time field use |
| Cloud-based point-cloud platforms | Matterport, Autodesk ReCap 360 | Upload required before any results; unusable with poor or zero connectivity; raw point cloud must leave the site |
| BIM-to-scan alignment tools | Trimble Connect, Autodesk Construction Cloud | Designed for desktop workflows, not edge deployment; require cloud round-trip; no integrated AI inference |
| General-purpose AI inspection | Various computer-vision SaaS platforms | Output is images with labels, not millimetre-level spatial deviation measurements; not integrated with BIM geometry |
| Traditional maritime survey | IACS paper-based procedures | No digital output; no comparison against prior surveys; not automated or scalable |

The common thread across all existing solutions is that they treat the scan, the AI analysis, and the BIM comparison as three separate steps performed in three separate tools, with a cloud upload between each. This is incompatible with the real-world constraints of field inspection: time pressure, connectivity limits, and the need for an on-site verdict.

---

## How EdgeSentry-Inspect is different

**Edge-first pipeline:** All computation — 3D projection, AI inference, BIM deviation, heatmap, report — runs on the field PC or robot. There is no cloud round-trip before the verdict. The system works with zero internet connectivity.

**Integrated flow:** The pipeline is a single continuous flow: point cloud → AI inference → deviation against BIM design → heatmap → JSON report. There are no hand-off steps between disconnected tools.

**Spatial precision:** Every anomaly is located in millimetres relative to the approved BIM design geometry. The report includes world-space coordinates, deviation magnitude, and AI classification — not just a photograph.

**Open and hardware-independent:** Built on open components: `trilink-core` (Rust), standard IFC files, any AI inference endpoint that accepts images. No proprietary sensor, cloud, or license required.

**Maritime-ready:** The pipeline handles offline buffering natively. The deviation log accumulates on the robot during a mission with zero connectivity, then syncs after docking. The report payload (1–6 MB) is sized for VDES terrestrial bandwidth, the IMO-standardised maritime data link used in port approaches and coastal waters.

**Optional cryptographic audit:** For high-assurance contexts — regulatory submissions, legally binding structural sign-off, maritime class certification — the deviation report can be signed with Ed25519 and hash-chained using [`edgesentry-rs`](https://github.com/edgesentry/edgesentry-rs). This produces an audit record that can be verified independently of EdgeSentry-Inspect infrastructure, with cryptographic proof that the report was not altered after the fact.
