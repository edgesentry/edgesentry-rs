# Roadmap

EdgeSentry-RS follows a phased approach: first establish the Singapore compliance baseline (CLS Level 2 → Level 3, SS 711:2025), then expand to Japan via GCLI mutual recognition (JC-STAR, Cyber Trust Mark), then achieve global convergence across EU, UK, and critical infrastructure markets. This mirrors the DuckDB model — build an embeddable OSS core that becomes a de facto standard through ecosystem adoption rather than lock-in.

## Why Singapore First

Singapore's CLS is directly derived from the European **ETSI EN 303 645** standard. Japan's JC-STAR similarly references ETSI EN 303 645 as its technical basis. This means the three regulatory regimes share a common foundation:

| Standard | Region | Based on |
|----------|--------|----------|
| ETSI EN 303 645 | Europe (CRA) | Original |
| CLS Level 2/3/4 | Singapore | ETSI EN 303 645 |
| JC-STAR | Japan | ETSI EN 303 645 |

By implementing Singapore CLS compliance first, the majority of the technical work directly satisfies Japan's JC-STAR and Europe's CRA requirements. The Singapore gateway is not just a regional target — it is the fastest path to global compliance coverage.

## GCLI and the Direct Japan-Singapore MoC

Japan signed the **Global Cyber Labelling Initiative (GCLI)** in 2025, joining 10 other countries including Singapore, UK, Finland, Germany, and Korea. GCLI establishes mutual recognition between national IoT security labels — a product certified under Singapore CLS is recognised as compliant with Japan's JC-STAR without re-certification. This is the structural mechanism that makes the "Singapore first" strategy work as a Japan entry path.

In March 2026, Japan and Singapore reinforced this with a **direct bilateral Memorandum of Cooperation (MoC)** between METI/IPA (Japan) and CSA (Singapore), establishing direct mutual recognition of JC-STAR and CLS labels. The MoC takes **effect on 1 June 2026**. Under this arrangement a valid, current JC-STAR label is accepted as-is under CLS — no re-derivation of CLS compliance from JC-STAR data is required. Japan is the fifth country to achieve bilateral mutual recognition with Singapore CLS (after Finland, Germany, South Korea, and the UK).

> **Open question:** The official level equivalence table mapping JC-STAR levels (STAR-1 through STAR-4) to CLS star levels (1–4) has not yet been published by CSA/METI. Monitor the CSA CLS page and METI/IPA JC-STAR page for this detail — it determines which JC-STAR level satisfies a given CLS target level.

Additional bilateral MRAs exist between Singapore CLS and Finland, Germany, and Korea. For Japanese customers already holding German or Korean IoT certification, these MRAs provide a fast-track CLS path.

## SS 711:2025 Design Principles

Singapore's national IoT standard SS 711:2025 (which replaces TR 64:2018 and underpins CLS Level 3 assessments) defines four security design principles. EdgeSentry-RS is designed around these:

| Principle | Requirement | Implementation |
|-----------|-------------|----------------|
| Secure by Default | Unique device identity, signed OTA | `identity.rs` (Ed25519), `update.rs` (signed update verification) |
| Rigour in Defence | STRIDE threat modelling, tamper detection | `integrity.rs` (BLAKE3 hash chain), STRIDE threat model artifacts |
| Accountability | Audit trail, operation logs | `ingest/` (AuditLedger, OperationLog, IntegrityPolicyGate) |
| Resiliency | Deny-by-default networking, rate limiting | `ingest/network_policy.rs` (IP/CIDR allowlist) |

## Implementation Mapping

For the detailed clause-by-clause mapping of CLS / ETSI EN 303 645 / JC-STAR requirements to source code, see the [Compliance Traceability Matrix](../security/cls-traceability.md).

---

## OSS scope

This repository implements the OSS audit layer: Ed25519 signing, BLAKE3 hash chain, ISO 19650 schema, and the `eds` verification CLI. All milestones in this document are open-source.

Commercial connectors (immugate WORM storage, CLS/JC-STAR compliance module, HSM key storage) are tracked in the commercial compliance layer.

---

## Phase 1: The Singapore Gateway (Current – 6 Months)

**Target:** CLS Level 2 → Level 3, SS 711:2025, iM8

Deliver a software reference implementation that satisfies Singapore CLS Level 2 cyber hygiene requirements and advances to Level 3 with the SDL evidence artifacts (threat model, SBOM, binary analysis) that IMDA assessors require.

### Milestone 1.1: Identity & Integrity Core ✅ Implemented

- `edgesentry_rs::identity` — Ed25519 device signature implementation
- `edgesentry_rs::integrity` — BLAKE3 hash chain tamper-detection protocol
- `edgesentry_rs::ingest::NetworkPolicy` — deny-by-default IP/CIDR allowlist (CLS-06)

### Milestone 1.2: The C/C++ Bridge ✅ Implemented

- `edgesentry-bridge` — C-compatible FFI layer exposing Ed25519 signing, signature verification, and hash-chain validation to C/C++ projects
- **Goal:** inject Singapore-grade security into existing Japanese hardware (gateways, sensors) with minimal modification
- See [C/C++ FFI Bridge](ffi_bridge.md) for usage, linking instructions, and memory safety conventions

### Milestone 1.3: Compliance Mapping v1.0 ✅ Implemented

- Traceability matrix mapping Singapore CLS/iM8 clauses to source code: [Compliance Traceability Matrix](../security/cls-traceability.md)

### Milestone 1.4: SBOM + Vendor Disclosure Checklist ✅ Implemented

IMDA's IoT Cyber Security Guide requires a vendor disclosure checklist as CLS Level 3 assessment evidence. The five mandatory categories are: encryption support, identification and authentication, data protection, network protection, and lifecycle support (SBOM).

- CycloneDX JSON SBOM generated for all crates and published with each GitHub Release
- Vendor disclosure checklist responses documented for all five categories
- Responses mapped to implementation in the traceability matrix
- See [SBOM and Vendor Disclosure](../security/sbom.md) and [#92](https://github.com/edgesentry/edgesentry-rs/issues/92)

### Milestone 1.5: Transport Layer, Async Ingest & Offline Buffer ✅ Implemented

- `async-ingest` feature: `AsyncIngestService<R,L,O>` with `&self` signature for safe multi-task sharing via `Arc` — closed [#115](https://github.com/edgesentry/edgesentry-rs/issues/115)
- `transport-http` feature: axum-based `POST /api/v1/ingest` endpoint; source IP gated through `NetworkPolicy` before crypto verification; `eds serve` CLI — closed [#116](https://github.com/edgesentry/edgesentry-rs/issues/116)
- `transport-tls` feature: `serve_tls()` with rustls TLS 1.2/1.3; `eds serve-tls --tls-cert / --tls-key` CLI; satisfies CLS-05 HTTP channel confidentiality — closed [#176](https://github.com/edgesentry/edgesentry-rs/issues/176)
- `transport-mqtt-tls` feature: `MqttTlsConfig` with CA cert path, rustls-backed MQTTS via rumqttc; `eds serve-mqtt --tls-ca-cert` CLI; satisfies CLS-05 MQTT channel confidentiality — closed [#180](https://github.com/edgesentry/edgesentry-rs/issues/180)
- `transport-mqtt` feature: `serve_mqtt()` subscribes to a configurable topic, routes records through `AsyncIngestService`, publishes accept/reject to `<topic>/response`; `eds serve-mqtt` CLI — closed [#146](https://github.com/edgesentry/edgesentry-rs/issues/146)
- `buffer` module: `OfflineBuffer<S>` store-and-forward with pluggable `BufferStore` trait; `InMemoryBufferStore` default; `SqliteBufferStore` behind `buffer-sqlite` feature; satisfies CLS-09 resilience — closed [#74](https://github.com/edgesentry/edgesentry-rs/issues/74)

### Milestone 1.6: STRIDE Threat Model + Binary Analysis Evidence ✅ Implemented

CLS Level 3 assessors expect recorded design artifacts, not just code. SS 711:2025 requires STRIDE-based threat modelling of all attack surfaces (API, communication, storage).

- STRIDE threat model covering: Spoofing (device identity), Tampering (audit records), Repudiation (operation logs), Information Disclosure (payload storage), Denial of Service (network policy), Elevation of Privilege (ingest gate) — see [`docs/src/threat_model.md`](../security/threat-model.md)
- Binary analysis evidence confirming no known CVEs in shipped crates (`cargo-audit`, `cargo-deny`)
- Threat model mitigations linked to traceability matrix entries — see [`docs/src/traceability.md`](../security/cls-traceability.md) (Rigour in Defence updated ✅)
- Japanese translation available at `docs/ja/src/threat_model.md`
- Closed: [#93](https://github.com/edgesentry/edgesentry-rs/issues/93) via PR [#143](https://github.com/edgesentry/edgesentry-rs/pull/143)

---

## Phase 2: Japan Adaptation via GCLI (6 – 12 Months)

**Target:** CLS Level 4, JC-STAR STAR-1/2, Cyber Trust Mark / ISO 27001

### Milestone 2.0: Mutual Recognition Framework (GCLI + Japan-Singapore MoC) 🔲 Planned

Two complementary mechanisms enable Japan market entry without duplicate certification:

1. **GCLI** — the multilateral framework (10+ countries) underpinning the overall Singapore-first strategy.
2. **Direct Japan-Singapore MoC** (signed March 2026, effective **1 June 2026**) — bilateral mutual recognition between JC-STAR and CLS. A valid JC-STAR label is accepted as-is under CLS; no re-mapping of certification data is required.

Deliverables for this milestone:

- Compliance pathway guide covering both the GCLI route and the direct MoC route for Japan-based customers
- JC-STAR label validation and attestation module (`edgesentry_rs::compliance::jcstar`) — see [#121](https://github.com/edgesentry/edgesentry-rs/issues/121)
- CLS ↔ JC-STAR level equivalence table (pending publication by CSA/METI; monitor CSA and METI/IPA pages)
- MRA fast-track guidance for customers holding Finnish, German, or Korean IoT certification
- See [#94](https://github.com/edgesentry/edgesentry-rs/issues/94)

### Milestone 2.1: JC-STAR STAR-1/2 Alignment 🔲 Planned

- Self-checklist and implementation guidance based on Japan's IoT Product Security Conformity Assessment criteria
- See [#82](https://github.com/edgesentry/edgesentry-rs/issues/82)

### Milestone 2.2: Edge Intelligence 🔲 Planned

- `edgesentry-summary` — data summarisation logic for high-performance Japanese sensors over bandwidth-constrained links. See [#83](https://github.com/edgesentry/edgesentry-rs/issues/83)
- `edgesentry-detector` — local anomaly detection with signed audit evidence attached to results. See [#84](https://github.com/edgesentry/edgesentry-rs/issues/84)

### Milestone 2.3: Cross-Border Education Program 🔲 Planned

- Joint technical white paper to help Japanese companies bid on Singapore public-infrastructure projects
- See [#85](https://github.com/edgesentry/edgesentry-rs/issues/85)

### Milestone 2.4: Cyber Trust Mark / ISO 27001 Organisational Track 🔲 Planned

Singapore's Cyber Trust Mark becomes mandatory for Critical Information Infrastructure (CII) operators from 2026–27. It is the organisational counterpart to CLS (which is product-level). B2B and government customers in Singapore will increasingly require vendors to support this track.

- Map EdgeSentry-RS implementation evidence to Cyber Trust Mark assessment categories
- ISO 27001 control alignment documentation
- See [#95](https://github.com/edgesentry/edgesentry-rs/issues/95)

### Milestone 2.6: immugate WORM Storage Connector

Moved to the commercial compliance layer.

---

### Milestone 2.7: ISO 19650 Information Container Schema 🔲 Planned

ISO 19650 defines the framework for managing information over the whole life cycle of a built asset using BIM. This milestone reframes each audit record as an ISO 19650 **information container**, enabling interoperability with third-party BIM tools and positioning the edgesentry-rs audit chain as a de facto standard for construction inspection traceability.

- `edgesentry_rs::audit::iso19650` — information container payload schema (OSS)
- Structured BIM status transitions: WIP → Shared → Published, with signed state change records
- Conformant metadata fields (revision, suitability, classification) mapped to the existing hash-chain record format
- Interoperability documentation for third-party BIM tool integration
- This milestone is the audit-crate implementation of the ISO 19650 layer described in the [Inspect roadmap](../inspect/roadmap.md)

---

### Milestone 2.5: CLS(MD) — Medical Device Variant 🔲 Planned

Singapore launched CLS for Medical Devices (CLS(MD)) in October 2024. If medical IoT is a target market, specific variant requirements apply.

- CLS(MD) gap analysis against current implementation
- Medical device–specific requirements identification
- See [#96](https://github.com/edgesentry/edgesentry-rs/issues/96)

---

## Phase 3: Global Convergence — "The European Horizon" (12 – 24 Months)

**Target:** EU CRA, UK PSTI Act, IEC 62443-4-2 (CII/OT), CCoP 2.0

### Milestone 3.1: EU CRA Compliance Research 🔲 Planned

- Full mapping to **ETSI EN 303 645** as a passport for the European market
- The Singapore CLS foundation covers the majority of CRA requirements with minimal additional work

### Milestone 3.2: UK PSTI Act Alignment 🔲 Planned

The UK Product Security and Telecommunications Infrastructure (PSTI) Act aligns with ETSI EN 303 645 and became effective January 2026. Given CLS compliance, this requires near-zero additional implementation.

- Gap analysis between CLS Level 3 and UK PSTI requirements
- PSTI compliance statement documentation
- See [#97](https://github.com/edgesentry/edgesentry-rs/issues/97)

### Milestone 3.3: IEC 62443-4-2 + Hardware RoT 🔲 Planned

IEC 62443-4-2 governs component-level requirements for Critical Infrastructure (CII) and OT markets. It requires hardware Root of Trust (TPM/HSM), RBAC, and Privileged Access Management (PAM) — distinct from ETSI EN 303 645.

- IEC 62443-4-2 component requirement mapping
- HSM integration via `edgesentry-bridge` for hardware-backed key storage (CLS Level 4)
- RBAC/PAM design guidance for deployers
- See [#54](https://github.com/edgesentry/edgesentry-rs/issues/54) and [#98](https://github.com/edgesentry/edgesentry-rs/issues/98)

### Milestone 3.4: CCoP 2.0 / MTCS Tier 3 🔲 Planned

Singapore's Cybersecurity Code of Practice 2.0 (CCoP 2.0) is the operational compliance requirement for CII sectors. MTCS Tier 3 applies if the platform has cloud or SaaS components targeting government contracts.

- CCoP 2.0 operational requirement mapping
- MTCS Tier 3 applicability assessment for cloud deployment scenarios
- See [#99](https://github.com/edgesentry/edgesentry-rs/issues/99)

### Milestone 3.5: Formal Verification & Hardening 🔲 Planned

- Advanced memory safety and vulnerability hardening to withstand third-party binary analysis required for CLS Level 4

### Milestone 3.6: Reference Architecture for AI Robotics 🔲 Planned

- Reference design for tamper-evident decision auditing in autonomous mobile robots (AMR) and inspection drones

---

## Sustainable Ecosystem Strategy

Following the DuckDB model — a lightweight embeddable core that spreads via libraries rather than platforms:

1. **"In-Process" Security** — Embed as a library inside existing C++ applications regardless of OS or hardware, just as DuckDB embeds inside Python and Java processes.

2. **Open Compliance** — OSS the "how to achieve security" knowledge, so no single vendor controls the compliance pathway; the standard becomes public infrastructure.

3. **Collaborative Learning** — Provide a shared Rust codebase as a cross-company learning environment to develop the next generation of IoT security engineers.
