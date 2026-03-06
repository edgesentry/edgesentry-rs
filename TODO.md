# TODO

This file tracks the current implementation status of `immutable-trace-sample`.

## Implemented (confirmed in this repository)

- [x] Audit record model with hash/signature fields and record chaining (`ledger-core`)
- [x] Device-side signed record generation (`device-agent`)
- [x] CLI commands for signing/verifying records and chain verification (`audit-cli`)
- [x] Lift inspection demo chain generation via CLI (`demo-lift-inspection`)
- [x] Ingest-side verification flow with deduplication/sequence checks (`ingest-api`)
- [x] PostgreSQL persistence schema for audit records and operation logs (`db/init/001_schema.sql`)
- [x] Local backend stack with PostgreSQL + MinIO (`docker-compose.local.yml`)
- [x] Interactive end-to-end local demo script with step-by-step pause (`scripts/local_demo.sh`)
- [x] Tampering detection demo (tampered chain is rejected)
- [x] Audit/operation log inspection in local demo output
- [x] Unit/integration test suites across crates (workspace test commands documented in `AGENTS.md`)
- [x] One-shot verification script for unit tests + commercial-use OSS license check (`scripts/run_unit_and_license_check.sh`)
- [x] OSS license policy file for `cargo-deny` (`deny.toml`)

## Not Implemented Yet / Needs Hardening for Industrial Production

- [ ] Hardware-backed key storage (HSM/TPM/secure element) and key attestation flow  
	Hardware needed: cloud/network HSM (e.g., AWS CloudHSM, Azure Managed HSM), device TPM 2.0 module, or MCU secure element (e.g., ATECC608/SE050).
- [ ] Production-grade key lifecycle management (rotation, revocation, recovery)
- [ ] Device identity provisioning/PKI lifecycle automation  
	Hardware needed: per-device hardware root of trust (TPM 2.0 or secure element) for device key generation/storage and attestation.
- [ ] Strong service authentication/authorization model (RBAC, token lifecycle, least privilege)
- [ ] End-to-end mTLS and certificate automation between device/ingest/storage paths
- [ ] High-availability deployment architecture (DB/Object storage redundancy, failover)  
	Hardware/infra needed: multi-node servers/VMs across fault domains (and, for on-prem, redundant network/power paths).
- [ ] Backup/restore and disaster recovery runbooks with recovery objective targets
- [ ] Observability stack (metrics, dashboards, tracing, alerting) with SLO/SLA definitions
- [ ] Load/stress/soak testing and performance baselines for industrial throughput  
	Hardware needed: dedicated load-generator machines (or equivalent cloud instances) to emulate production-scale traffic.
- [ ] Security hardening pipeline (SAST/DAST/dependency scanning/signing) in CI/CD
- [ ] Formal compliance mapping and evidence process (audit retention/access policy controls)
- [ ] Multi-environment deployment automation (staging/prod infrastructure as code)
- [ ] Operational incident response playbooks and on-call procedures
- [ ] Data governance controls (retention windows, archival, deletion policy by regulation)

## Suggested Next Priorities

1. Integrate managed key management (KMS/HSM) and key rotation.
2. Add authentication/authorization and mTLS for ingest APIs.
3. Define HA + backup/restore strategy for PostgreSQL/MinIO equivalents.
4. Add observability + alerting and validate with failure drills.
5. Run load/soak tests and document production operating limits.
