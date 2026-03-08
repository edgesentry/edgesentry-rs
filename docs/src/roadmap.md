# Roadmap

EdgeSentry-RS follows a phased approach: first establish the Singapore regulatory baseline (CLS/iM8), then expand to Japan (JC-STAR) and Europe (CRA). This mirrors the DuckDB model — build an embeddable OSS core that becomes a de facto standard through ecosystem adoption rather than lock-in.

## Why Singapore First

Singapore's CLS is directly derived from the European **ETSI EN 303 645** standard. Japan's JC-STAR similarly references ETSI EN 303 645 as its technical basis. This means the three regulatory regimes share a common foundation:

| Standard | Region | Based on |
|----------|--------|----------|
| ETSI EN 303 645 | Europe (CRA) | Original |
| CLS Level 3/4 | Singapore | ETSI EN 303 645 |
| JC-STAR | Japan | ETSI EN 303 645 |

By implementing Singapore CLS compliance first, the majority of the technical work directly satisfies Japan's JC-STAR and Europe's CRA requirements. The Singapore gateway is not just a regional target — it is the fastest path to global compliance coverage.

---

## Phase 1: Foundation — "The Singapore Gateway" (Current – 6 Months)

Deliver a software reference implementation that satisfies Singapore CLS Level 3/4 and iM8 requirements.

### Milestone 1.1: Identity & Integrity Core

- `edgesentry-identity` — Ed25519 device signature implementation
- `edgesentry-integrity` — BLAKE3 hash chain tamper-detection protocol

### Milestone 1.2: The C/C++ Bridge

- `edgesentry-bridge` — FFI layer allowing C++ projects to call Rust signing and verification without a full rewrite
- **Goal:** inject Singapore-grade security into existing Japanese hardware (gateways, sensors) with minimal modification

### Milestone 1.3: Compliance Mapping v1.0

- Publish a traceability matrix mapping Singapore CLS/iM8 clauses to source code

---

## Phase 2: Japan Adaptation (6 – 12 Months)

Strengthen alignment with Japan's IoT security label scheme (JC-STAR) and unified government standards.

### Milestone 2.1: JC-STAR STAR-1/2 Alignment

- Self-checklist and implementation guidance based on Japan's IoT Product Security Conformity Assessment criteria

### Milestone 2.2: Edge Intelligence

- `edgesentry-summary` — data summarization logic for high-performance Japanese sensors (e.g., HMS AI cameras) over bandwidth-constrained links
- `edgesentry-detector` — local anomaly detection with signed audit evidence attached to results

### Milestone 2.3: Cross-Border Education Program

- Joint technical white paper to help Japanese companies bid on Singapore public-infrastructure projects

---

## Phase 3: Global Convergence — "The European Horizon" (12 – 24 Months)

Target the EU Cyber Resilience Act (CRA) and broader critical infrastructure (CI) sectors.

### Milestone 3.1: EU CRA Compliance Research

- Full mapping to **ETSI EN 303 645** as a passport for the European market

### Milestone 3.2: Formal Verification & Hardening

- Advanced memory safety and vulnerability hardening to withstand third-party binary analysis required for CLS Level 4

### Milestone 3.3: Reference Architecture for AI Robotics

- Reference design for tamper-evident decision auditing in autonomous mobile robots (AMR) and inspection drones

---

## Sustainable Ecosystem Strategy

Following the DuckDB model — a lightweight embeddable core that spreads via libraries rather than platforms:

1. **"In-Process" Security** — Embed as a library inside existing C++ applications regardless of OS or hardware, just as DuckDB embeds inside Python and Java processes.

2. **Open Compliance** — OSS the "how to achieve security" knowledge so no single vendor controls the compliance pathway; the standard becomes public infrastructure.

3. **Collaborative Learning** — Provide a shared Rust codebase as a cross-company learning environment to develop the next generation of IoT security engineers.
