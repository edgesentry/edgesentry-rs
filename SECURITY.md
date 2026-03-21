# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.x (current) | ✅ |

Only the latest published version on crates.io receives security updates. Patch releases are issued for confirmed vulnerabilities; update to the latest patch version as soon as possible.

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report security issues privately using [GitHub's private vulnerability reporting](https://github.com/edgesentry/edgesentry-rs/security/advisories/new).

Include as much of the following as possible:

- Description of the vulnerability and its potential impact
- Steps to reproduce or a minimal proof-of-concept
- Affected version(s) and component(s) (`edgesentry-rs`, `edgesentry-bridge`, etc.)
- Any suggested mitigations you have identified

### Response timeline

| Stage | Target |
| ----- | ------ |
| Acknowledgement | Within 3 business days |
| Triage and severity assessment | Within 7 business days |
| Patch or mitigation plan communicated | Within 30 days (critical / high); 90 days (medium / low) |
| Public disclosure | After patch is available and reporter is notified |

If a reported vulnerability is accepted, we will credit the reporter in the release notes unless they prefer to remain anonymous.

If a reported vulnerability is declined (e.g. out of scope, not reproducible, or working as intended), we will explain our reasoning within the response timeline above.

### Scope

In scope:

- Cryptographic correctness in `edgesentry-rs` (Ed25519 signing, BLAKE3 hashing, hash-chain verification)
- Ingest pipeline integrity and authentication controls
- FFI memory safety in `edgesentry-bridge`
- Supply-chain security issues in direct dependencies

Out of scope:

- Vulnerabilities in transitive dependencies not exploitable through `edgesentry-rs`'s public API
- Issues requiring physical device access (report to the relevant hardware vendor)
- Denial-of-service issues with no cryptographic or integrity impact
