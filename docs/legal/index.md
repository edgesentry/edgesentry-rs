# Legal Admissibility of Audit Logs — edgesentry-rs

- **Updated:** 2026-04-30  
- **Scope:** Maritime compliance, workplace safety, insurance claims — Singapore-primary, internationally relevant

An audit log that cannot be admitted as evidence is commercially worthless. This document defines the seven requirements for legal admissibility, maps each to the current edgesentry-rs implementation, and identifies gaps with mitigations.

---

## Applicable legal frameworks

| Framework | Relevance |
|---|---|
| Singapore [**Evidence Act** (Cap. 97), s.116A](https://sso.agc.gov.sg/Act/97) | Computer-generated records admissible if system was "operating properly" |
| Singapore [**Electronic Transactions Act** (Cap. 88), s.8](https://sso.agc.gov.sg/act/eta2010) | Electronic record = reliable if integrity can be shown |
| Singapore [**Workplace Safety and Health Act**](https://sso.agc.gov.sg/Act/WSHA2006) | Incident records; MOM enforcement |
| [**IMO FAL Convention**](https://www.imo.org/en/about/conventions/pages/convention-on-facilitation-of-international-maritime-traffic-(fal).aspx) Annex | Electronic documents equivalent to paper if integrity is demonstrable |
| [**BWM Convention**](https://www.imo.org/en/about/conventions/pages/international-convention-for-the-control-and-management-of-ships'-ballast-water-and-sediments-(bwm).aspx) Reg. B-2 | Ballast Water Record Book — 5-year retention minimum |
| [**MLC 2006**](https://www.ilo.org/international-labour-standards/maritime-labour-convention-2006) Standard A5.2.1 | Port State Control inspection records |
| [**Marine Insurance Act 1906**](https://www.legislation.gov.uk/ukpga/Edw7/6/41/contents) (UK, applicable in SG) | Uberrimae fidei — materiality of pre-loss documentation |
| [**UNCITRAL Model Law on Electronic Commerce**](https://uncitral.un.org/en/texts/ecommerce/modellaw/electronic_commerce) | International framework; functional equivalence principle |

---

## Requirement 1 — Integrity: the record was not altered after creation

**What must be shown:** Content of each record is provably unchanged from creation.

**edgesentry-rs implementation:**
- `payload_hash` = BLAKE3 hash of canonical payload bytes
- Ed25519 signature over `payload_hash`
- `prev_record_hash` links each record to its predecessor
- `eds audit verify-chain` recomputes all hashes and verifies all signatures

**Status: ✓ Met**

---

## Requirement 2 — Attribution: who created the record

**What must be shown:** Identify the device and operator that produced the record.

**edgesentry-rs implementation:**
- `device_id` field in every `AuditRecord`
- Ed25519 signing key is per-device; only the holder of the private key produces a valid signature
- Public key registered in `IntegrityPolicyGate` at device onboarding

**Status: ✓ Met (Phase 1)**

**Gap:** Private key custody. If the operator controls the key, they could sign false records.

**Mitigation — Phase 1:** Key registration at onboarding — customer's public key recorded by edgesentry with timestamp. Any record signed by key K after date D is attributable to site S.

**Mitigation — Phase 2:** HSM or TPM on the edge device — private key never extractable, even by the operator. Tracked in [#54](https://github.com/edgesentry/edgesentry-rs/issues/54).

---

## Requirement 3 — Trusted timestamp: when the record was created

**What must be shown:** Creation time from a source independent of the operator.
A device's local clock is not trusted in legal proceedings if the operator can manipulate it.

**edgesentry-rs implementation:**
- `timestamp_ms` = `SystemTime::now()` at signing time
- This is the device's local clock — operator-controlled

**Status: ✗ Gap — the single weakest point**

**Mitigation A — Phase 1 (immediate):**
Upload to Cloudflare R2 immediately on creation. R2 stores an immutable
`x-amz-date` header set by Cloudflare's servers, not the device. Cloudflare is
a neutral third party. Argument: "the device signed the record; Cloudflare
independently timestamped the upload; both timestamps agree within N seconds."

**Mitigation B — Phase 2:**
[RFC 3161](https://www.rfc-editor.org/rfc/rfc3161.html) Timestamp Authority (TSA).
After local signing, the record hash is submitted to a TSA (DigiCert, GlobalSign, etc.).
The TSA returns a signed timestamp token stored alongside the record. TSA timestamps
are legally recognised in most jurisdictions — the standard for long-term validation
under [eIDAS](https://eur-lex.europa.eu/eli/reg/2014/910/oj/eng) PAdES-LT.

---

## Requirement 4 — Completeness: no records deleted or skipped

**What must be shown:** No gaps in the record; an operator cannot delete inconvenient events.

**edgesentry-rs implementation:**
- `sequence` field is monotonically increasing (1, 2, 3 …)
- `prev_record_hash` of record N equals the hash of record N-1
- Deleting record N breaks the chain at N+1

**Status: ✓ Met**

**Note:** Completeness holds only if the verifier has the full chain from sequence 1.
Partial exports must include an anchor record containing the `prev_record_hash` so the
verifier can confirm it connects to the previously known chain tail.

---

## Requirement 5 — Non-repudiation: signer cannot deny having signed

**What must be shown:** The party that produced the record cannot credibly deny it.

**edgesentry-rs implementation:**
- Ed25519 is asymmetric — only the private key holder produces a valid signature for a given public key
- Public key registered with edgesentry at onboarding (see Requirement 2)

**Status: ✓ Met (conditionally)**

**Gap:** Shared keys. If multiple devices share one private key, non-repudiation weakens
to "someone at site S" rather than "device D."

**Mitigation:** Enforce one key pair per physical device at onboarding. Stored as `device_id → public_key_hex` in the key registry.

---

## Requirement 6 — System integrity: the software was unmodified

**What must be shown (Evidence Act s.116A):** The system was "operating properly."
This means the software version that produced each record must be identifiable.

**edgesentry-rs implementation:**
- `profile_version` in `MeasurementRecord` — identifies the active rule set
- `device_id` identifies the device

**Missing:** `software_version` field in `AuditRecord` — no build hash today.

**Status: ⚠ Partial**

**Mitigation:** Add `software_version: String` to `AuditRecord` — embed the `eds` binary's
Git SHA or release tag at compile time via `env!("CARGO_PKG_VERSION")` + build metadata.
Tracked in the [Roadmap](../roadmap/index.md).

---

## Requirement 7 — Retention and retrievability

**What must be shown:** Records accessible when needed.
BWM Convention requires 5-year retention; MOM/WSH inspections can occur years after an incident.

**edgesentry-rs implementation:**
- R2 Object Lock (Compliance mode) — records cannot be deleted or overwritten for the retention period
- JSON format with open algorithms (BLAKE3, Ed25519) — no proprietary software required to verify

**Status: ✓ Met (once R2 Object Lock is configured)**

**Gap:** Format longevity. Algorithm identifiers are not embedded in the record schema today.

**Mitigation:** Add `"hash_alg": "blake3"` and `"sig_alg": "ed25519"` to `AuditRecord`.
Publish `eds audit verify-chain` as open-source (Apache 2.0) so independent verifiers
can re-implement.

---

## Summary matrix

| Requirement | Status | Gap | Phase 1 mitigation | Phase 2 mitigation |
|---|---|---|---|---|
| 1. Integrity | ✓ Met | — | — | — |
| 2. Attribution | ✓ Phase 1 | Key custody | Key registration at onboarding | HSM / TPM ([#54](https://github.com/edgesentry/edgesentry-rs/issues/54)) |
| 3. Trusted timestamp | ✗ Gap | Local clock | R2 upload timestamp (Cloudflare anchor) | RFC 3161 TSA token |
| 4. Completeness | ✓ Met | Partial exports | Anchor record in every export | — |
| 5. Non-repudiation | ✓ Conditional | Shared keys | One key per device at onboarding | — |
| 6. System integrity | ⚠ Partial | No `software_version` | Add Git SHA to `AuditRecord` | — |
| 7. Retention | ✓ Met (R2) | Algorithm IDs | Add `hash_alg`/`sig_alg` fields | — |

---

## Comparison with paper-based maritime records

| Property | Paper logbook | edgesentry-rs Phase 1 |
|---|---|---|
| Tamper detection | None (ink alterable) | BLAKE3 hash chain |
| Attribution | Handwritten signature | Ed25519 + `device_id` |
| Timestamp | Officer's handwriting | Local clock (same as paper) |
| Completeness | Pages removable | Chain break detectable |
| Retention | Physical storage (fire/flood risk) | R2 Object Lock (geo-redundant) |
| Retrievability | Manual search | Queryable by `device_id`, timestamp range |

The current implementation already exceeds paper-based practice on integrity, completeness, and retention. The timestamp dimension is equivalent to paper in Phase 1 (operator-controlled clock), and surpasses it in Phase 2 (RFC 3161 TSA).

---

## Implementation roadmap

**Before PIER71 / CAP Vista submission (June 2026):**
- [ ] Add `software_version` (Git SHA) field to `AuditRecord` in `edgesentry-audit`
- [ ] Add `hash_alg` and `sig_alg` fields to `AuditRecord`
- [ ] Document key registration process and R2 upload timestamp anchor

**Phase 2 (post-submission, Nov 2026 PoC):**
- [ ] [RFC 3161](https://www.rfc-editor.org/rfc/rfc3161.html) TSA integration — submit record hash to TSA on signing, store token alongside record
- [ ] HSM / TPM support on edge device ([#54](https://github.com/edgesentry/edgesentry-rs/issues/54))
- [ ] Partial chain export format — anchor record + proof of connection to root

**Before production / insurance partnership:**
- [ ] External legal opinion from a Singapore maritime law firm on Evidence Act s.116A compliance
- [ ] Pilot audit with one P&I club or H&M underwriter to confirm evidence requirements

---

## Relationship to security standards

Legal admissibility overlaps with but is distinct from IoT security certification:

- **IoT security standards** (CLS, ETSI EN 303 645, JC-STAR) define *how* a system should be built — access control, update integrity, network policy. See [`docs/security/`](../security/index.md).
- **Legal admissibility** defines *what a court or regulator will accept as evidence* — integrity, attribution, trusted timestamp, completeness, non-repudiation. This document covers that.

The two are complementary: a CLS Level 3-certified system is a strong foundation for
a legally admissible audit record, but certification alone does not guarantee admissibility.
