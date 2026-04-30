# IoT Security Standards — edgesentry-rs

EdgeSentry-RS is designed to satisfy the following IoT security certification
standards. This folder is the single source for compliance evidence, threat
modelling, and SBOM artifacts that span the full workspace.

---

## Targeted standards

| Standard | Body | Scope | Level targeted |
|---|---|---|---|
| [SS 711:2025](https://www.singaporestandardseshop.sg/) | Singapore Standards Council | National IoT cybersecurity standard; four design principles | All four principles |
| [CLS Level 3 / Level 4](https://www.csa.gov.sg/our-programmes/certification-and-labelling-schemes/cybersecurity-labelling-scheme) | Cyber Security Agency (CSA) Singapore | Cybersecurity Labelling Scheme for IoT products | Level 3 (current), Level 4 (planned) |
| [ETSI EN 303 645](https://www.etsi.org/deliver/etsi_en/303600_303699/303645/02.01.01_60/en_303645v020101p.pdf) | ETSI | European IoT cybersecurity baseline; 13 provisions | All 13 provisions mapped |
| [iM8](https://www.imda.gov.sg/regulations-and-licensing-listing/IMDA-Standards-Collection/iM8) | IMDA Singapore | IoT Cyber Security Guide; vendor disclosure checklist | Full checklist |
| [JC-STAR](https://www.soumu.go.jp/main_sosiki/cybersecurity/jc-star/) | MIC Japan | Japan's IoT security standard (STAR-1 / STAR-2) | STAR-1 + STAR-2 mapped |
| [IMO MSC.428(98)](https://www.imo.org/en/OurWork/Security/Pages/Cyber-security.aspx) | IMO | Maritime cyber risk management in Safety Management Systems | Reference only |

---

## Document map

| Document | Contents |
|---|---|
| [STRIDE Threat Model](threat-model.md) | Full attack-surface analysis: Spoofing, Tampering, Repudiation, Information Disclosure, DoS, Elevation of Privilege — mapped to source code |
| [CLS / ETSI / JC-STAR Traceability Matrix](cls-traceability.md) | Clause-by-clause mapping of CLS Level 3/4, ETSI EN 303 645, iM8, and JC-STAR requirements to implementation |
| [SBOM and Vendor Disclosure Checklist](sbom.md) | Software Bill of Materials (CycloneDX format), supply-chain monitoring, and IMDA vendor disclosure checklist responses |

---

## SS 711:2025 — four design principles

| Principle | Requirement | Status | Implementation |
|---|---|---|---|
| Secure by Default | Unique device identity, signed OTA updates | ✅ | `identity.rs`, `update.rs` |
| Rigour in Defence | STRIDE threat model, tamper detection | ✅ | Hash chain (`integrity.rs`) + [threat model](threat-model.md) |
| Accountability | Audit trail, operation logs | ✅ | `ingest/` (AuditLedger, OperationLog) |
| Resiliency | Deny-by-default networking, DoS protection | ✅ | `ingest/network_policy.rs` |

---

## Coverage summary

| Level | Total clauses | ✅ Implemented | ⚠️ Partial | 🔲 Planned | ➖ Out of scope |
|---|---|---|---|---|---|
| CLS Level 3 | 11 | 6 | 2 | 0 | 3 |
| CLS Level 4 | 1 | 0 | 0 | 1 | 0 |
| JC-STAR additions | 1 | 1 | 0 | 0 | 0 |

Full clause-by-clause breakdown: [CLS / ETSI / JC-STAR Traceability Matrix](cls-traceability.md).

---

## Relationship to other doc folders

- **`docs/audit/`** — audit crate internals: `AuditRecord` design, key management, CLI, deployment
- **`docs/legal/`** — legally admissible audit log requirements (7-requirement analysis, trusted timestamp, RFC 3161 TSA roadmap)
- **`docs/pipeline/`** — seven-step pipeline and edge/cloud split design
