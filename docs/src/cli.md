# CLI Reference

Full CLI usage, lift inspection scenario, and tampering walkthrough are in [AGENTS.md](https://github.com/yohei1126/edgesentry-rs/blob/main/AGENTS.md).

Key sections:

- **CLI Usage** — `sign-record`, `verify-record`, `verify-chain` commands with examples
- **Lift Inspection Scenario (CLI End-to-End)** — generate a signed chain, verify it, tamper and confirm detection
- **S3 / MinIO Switching** — `S3ObjectStoreConfig::for_aws_s3(...)` vs `S3ObjectStoreConfig::for_minio(...)`
