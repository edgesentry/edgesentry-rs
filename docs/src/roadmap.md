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

## GCLI: The Bridge to Japan and Beyond

Japan signed the **Global Cyber Labelling Initiative (GCLI)** in 2025, joining 10 other countries including Singapore, UK, Finland, Germany, and Korea. GCLI establishes mutual recognition between national IoT security labels — a product certified under Singapore CLS is recognised as compliant with Japan's JC-STAR without re-certification. This is the mechanism that makes the "Singapore first" strategy work as a Japan entry path.

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

For the detailed clause-by-clause mapping of CLS / ETSI EN 303 645 / JC-STAR requirements to source code, see the [Compliance Traceability Matrix](traceability.md).

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

- Traceability matrix mapping Singapore CLS/iM8 clauses to source code: [Compliance Traceability Matrix](traceability.md)

### Milestone 1.4: SBOM + Vendor Disclosure Checklist 🔲 Planned

IMDA's IoT Cyber Security Guide requires a vendor disclosure checklist as CLS Level 3 assessment evidence. The five mandatory categories are: encryption support, identification and authentication, data protection, network protection, and lifecycle support (SBOM).

- Generate and publish SBOM (Software Bill of Materials) for all crates
- Document vendor disclosure checklist responses for each category
- Map checklist responses to existing implementation in the traceability matrix
- See [#92](https://github.com/edgesentry/edgesentry-rs/issues/92)

### Milestone 1.5: STRIDE Threat Model + Binary Analysis Evidence 🔲 Planned

CLS Level 3 assessors expect recorded design artifacts, not just code. SS 711:2025 requires STRIDE-based threat modelling of all attack surfaces (API, communication, storage).

- STRIDE threat model covering: Spoofing (device identity), Tampering (audit records), Repudiation (operation logs), Information Disclosure (payload storage), Denial of Service (network policy), Elevation of Privilege (ingest gate)
- Binary analysis report confirming no known CVEs in shipped crates (`cargo-audit`, `cargo-deny`)
- Link threat model mitigations to traceability matrix entries
- See [#93](https://github.com/edgesentry/edgesentry-rs/issues/93)

---

## Phase 2: Japan Adaptation via GCLI (6 – 12 Months)

**Target:** CLS Level 4, JC-STAR STAR-1/2, Cyber Trust Mark / ISO 27001

### Milestone 2.0: GCLI Mutual Recognition Framework 🔲 Planned

GCLI is the primary mechanism for Japan market entry without duplicate certification. Document the CLS → JC-STAR equivalence mapping under GCLI, and provide guidance for Japanese hardware vendors on leveraging existing MRAs (Finland, Germany, Korea bilateral agreements).

- GCLI compliance pathway guide for Japan-based customers
- CLS ↔ JC-STAR clause equivalence table
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
