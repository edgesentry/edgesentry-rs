# Security & Compliance

edgesentry-rs targets the following certifications. Each document below is a direct evidence artifact for the corresponding assessment.

| Standard | Body | Target level |
|---|---|---|
| CLS | CSA Singapore | Level 3 (current), Level 4 (planned) |
| SS 711:2025 | Singapore Standards Council | All four design principles |
| ETSI EN 303 645 | ETSI | All 13 provisions |
| iM8 | IMDA Singapore | Full vendor disclosure checklist |
| JC-STAR | MIC Japan | STAR-1 + STAR-2 |

## Evidence documents

| Document | What it proves | Assessor use |
|---|---|---|
| [threat-model.md](threat-model.md) | Design validity — STRIDE analysis mapped to source code | SS 711:2025 Rigour in Defence; ETSI Provision 5.3 |
| [compliance-matrix.md](compliance-matrix.md) | Implementation coverage — clause-by-clause mapping of CLS / ETSI / JC-STAR to code | Primary reference for CLS Level 3/4 assessment |
| [key-management.md](key-management.md) | Key lifecycle — generation, storage (HSM/Vault/env), rotation | CLS-04; ETSI Provision 5.4 |
| [sbom-lifecycle.md](sbom-lifecycle.md) | Supply chain — SBOM (CycloneDX), dependency monitoring, vulnerability SLA, IMDA disclosure checklist | ETSI Provision 5.7; iM8 vendor disclosure |

## SS 711:2025 — design principle status

| Principle | Status | Evidence |
|---|---|---|
| Secure by Default | ✅ | [compliance-matrix.md](compliance-matrix.md) |
| Rigour in Defence | ✅ | [threat-model.md](threat-model.md) |
| Accountability | ✅ | [compliance-matrix.md](compliance-matrix.md) |
| Resiliency | ✅ | [compliance-matrix.md](compliance-matrix.md) |

## Related

- **[docs/roadmap/strategy-compliance.md](../roadmap/strategy-compliance.md)** — phased plan for SG → JP → EU market compliance
- **[docs/legal.md](../legal.md)** — legal admissibility of audit records (Evidence Act, RFC 3161)
- **[docs/pipeline/](../pipeline/)** — edge/cloud split and CV adapter contract
