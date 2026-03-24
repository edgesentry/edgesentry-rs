# EdgeSentry-Inspect — Requirements

For deep-dive step-by-step flows, case studies, and implementation order, see [scenarios.md](scenarios.md).

## Use cases

### UC-1: Construction site inspection

| Item | Detail |
|---|---|
| **Trigger** | Inspector arrives on site with a 3D sensor device |
| **Constraint** | Full scan and verdict for one unit within **30 minutes** |
| **Output** | Pass / fail verdict per element; deviation heatmap; deviation report |
| **Regulatory target** | CONQUAS automated inspection criteria |
| **Data flow** | Scan → edge PC → verdict displayed on tablet; report uploaded to common data environment |

The 30-minute constraint makes a cloud round-trip infeasible. All computation from point cloud to deviation report must complete on the field PC.

### UC-2: Maritime structure inspection

| Item | Detail |
|---|---|
| **Trigger** | Autonomous robot completes a hull or confined-space scan mission |
| **Constraint** | Intermittent or zero connectivity during the mission |
| **Output** | Structural-change flags (real-time, edge); full deviation report (post-mission, cloud) |
| **Regulatory target** | Maritime Digital Twin integration |
| **Data flow** | Robot scans → edge pipeline → flag emitted if anomaly exceeds threshold → report synced to central system after mission |

---

## KPIs

| KPI | Target | Rationale |
|---|---|---|
| Inspection time reduction | ≥ 50% vs manual | Replaces manual measurement per element |
| Labour reduction | ≥ 80% vs manual | Automated deviation computation and reporting |
| Productivity improvement | ≥ 20% overall | Minimum threshold required by regulatory programmes |
| Deviation detection accuracy | ≤ 5 mm error | Structural tolerance for concrete and steel elements |
| Time to verdict (UC-1) | ≤ 30 min per unit | Hard constraint from CONQUAS automated inspection programme |
| Report upload latency (UC-2) | Best-effort; no hard limit during mission | Mission-critical flag delivered immediately; report synced after |

---

## Non-functional requirements

| Requirement | Detail |
|---|---|
| **Offline operation** | Edge pipeline must work with zero internet connectivity |
| **Immutability** | Uploaded reports must be stored in append-only, tamper-evident storage (Object Lock WORM) |
| **Auditability** | Every deviation report must carry a timestamp, sensor serial, and IFC model reference |
| **No raw point cloud upload** | Only the deviation report is uploaded — reduces bandwidth and avoids data sovereignty issues |
| **Hardware independence** | Edge pipeline must run on a standard field PC with a consumer GPU; no proprietary cloud hardware |

---

## Accuracy requirements by scenario

### UC-1: Construction site inspection

| Parameter | Target | Driver |
|---|---|---|
| Deviation detection threshold | 10 mm | Structural concrete tolerance |
| Position accuracy of anomaly location | ≤ 10 mm | Consistent with deviation threshold |
| False positive rate | < 5% | Inspector must trust the system; too many false flags causes rejection |
| Coverage report | ≥ 80% of design surface scanned | Partial scans produce misleading `compliant_pct` if coverage is not reported |

### UC-2: Maritime structure inspection

| Parameter | Target | Driver |
|---|---|---|
| Deviation detection threshold | 5 mm | Hull deformation tolerance; structural safety standard |
| Position accuracy of anomaly location | ≤ 5 mm | Consistent with deviation threshold |
| Mission duration without connectivity | Up to 4 hours | Confined hull inspection mission length |
| Sync latency after docking | < 5 minutes | Control centre needs updated state promptly after mission |
