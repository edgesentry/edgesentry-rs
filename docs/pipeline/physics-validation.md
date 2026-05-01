# Physics Engine Validation

This document records the scenario-level ground-truth validation of the edgesentry-rs physics engine.
Each scenario is defined analytically; expected outputs are hand-calculated and asserted in
`crates/edgesentry-evaluate/src/rules.rs` (the `scenario_*` tests).

Run with: `cargo test -p edgesentry-evaluate scenario`

---

## Scenario 1 — Proximity approach (port safety)

**Setup:**
- Worker W-01 stationary at origin (0, 0)
- Vehicle FL-01 closes from 12.0 m to 1.0 m over 15 frames at constant velocity
- Step = (12.0 − 1.0) / 14 = **11/14 ≈ 0.786 m/frame** (1 frame = 1 s)
- Rule: `PROXIMITY_ALERT` — `distance < 5.0 m`

**Hand-calculated ground truth:**

First alert frame: `12.0 − N × (11/14) < 5.0` → `N > 98/11 = 8.909` → **frame 9**

Distance at frame 9: `12.0 − 9 × (11/14) = 69/14 ≈ 4.929 m`

First TTC alert frame: `TTC = distance/speed = (12 × 14/11) − N < 3.0` → `N > 135/11 = 12.27` → **frame 13**

TTC at frame 13: `(25/14) / (11/14) = 25/11 ≈ 2.273 s`

**Assertions:**
- `PROXIMITY_ALERT` first fires at frame 9; `measured_value = 69/14 ≈ 4.929 m`
- `TTC_ALERT` first fires at frame 13; `measured_value = 25/11 ≈ 2.273 s`

---

## Scenario 2 — TTC trigger (clean numbers)

**Setup:**
- Worker W-01 stationary at (0, 0)
- Vehicle FL-01 at (8.0, 0), velocity (−4.0, 0) m/s
- Rules: `PROXIMITY_ALERT` (`distance < 5.0`) + `TTC_ALERT` (`ttc < 3.0`)

**Hand-calculated ground truth:**

`TTC = distance / closing_speed = 8.0 / 4.0 = 2.0 s` → below threshold 3.0 s → **TTC_ALERT fires**

`distance = 8.0 m` → above threshold 5.0 m → **PROXIMITY_ALERT silent**

**Assertions:**
- `TTC_ALERT` fires; `measured_value = 2.0 s`, `threshold = 3.0 s`
- `PROXIMITY_ALERT` does not fire

---

## Scenario 3 — Safe pass, zero false positives

**Setup:**
- Worker W-01 stationary at (0, 0)
- Vehicle FL-01 moves parallel at y = 6.0 m, velocity (1.0, 0) m/s — no closing component
- 10 frames evaluated

**Hand-calculated ground truth:**

`distance = sqrt(x² + 36) ≥ 6.0 m` at all frames → above threshold 5.0 m → no proximity alert

Closing speed = 0 (orthogonal motion) → TTC = ∞ → no TTC alert

**Assertion:** Zero events across all 10 frames.

---

## Scenario 4 — Zone boundary precision

**Setup:**
- Zone polygon: `[[300,200],[600,200],[600,500],[300,500]]` (300 m × 300 m)
- Vessel A at (299, 350): x < 300 → outside
- Vessel B at (301, 350): x ∈ [300,600], y ∈ [200,500] → inside

**Hand-calculated ground truth:**

Point-in-polygon test (ray casting):
- (299, 350): ray crosses 0 zone edges → **outside → no alert**
- (301, 350): ray crosses 1 zone edge → **inside → ZONE_ENTRY fires**

**Assertions:**
- Vessel A: no events
- Vessel B: `ZONE_ENTRY` fires

---

## Scenario 5 — Zone exit: events on inside frames only

**Setup:**
- Zone: `[[300,200],[600,200],[600,500],[300,500]]`
- Vessel trajectory (Δt = 30 s, speed = 2 m/s → 60 m/frame):

| Frame | t (ms) | x | Inside zone? | Expected |
|---|---|---|---|---|
| 0 | 0 | 250 | No (x < 300) | silent |
| 1 | 30 000 | 310 | Yes | `ZONE_ENTRY` |
| 2 | 60 000 | 400 | Yes | `ZONE_ENTRY` |
| 3 | 90 000 | 500 | Yes | `ZONE_ENTRY` |
| 4 | 120 000 | 590 | Yes | `ZONE_ENTRY` |
| 5 | 150 000 | 650 | No (x > 600) | silent |

**Assertion:** `ZONE_ENTRY` fires at frames 1–4 only; frames 0 and 5 produce no events.

---

## Summary

| Scenario | Rule(s) tested | Key assertion |
|---|---|---|
| 1a Proximity approach | `PROXIMITY_ALERT` | First alert at exact frame 9; `measured_value = 69/14 m` |
| 1b TTC escalation | `TTC_ALERT` | First alert at exact frame 13; `measured_value = 25/11 s` |
| 2 TTC clean numbers | `TTC_ALERT` | `TTC = 8/4 = 2.0 s`; proximity silent at 8 m |
| 3 Safe pass | both | Zero events across 10 frames |
| 4 Zone boundary | `ZONE_ENTRY` | Fires at x=301, silent at x=299 |
| 5 Zone exit | `ZONE_ENTRY` | Fires on 4 inside frames; silent on entry and exit frames |

All scenarios pass: `cargo test -p edgesentry-evaluate scenario`
