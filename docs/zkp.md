# Zero-Knowledge Proofs in EdgeSentry

- **Updated:** 2026-05-09
- **Status:** Mock framework active; SP1 (Succinct) is the production target

---

## What is a zero-knowledge proof?

A zero-knowledge proof (ZKP) lets one party (the **prover**) convince another party (the **verifier**) that a statement is true — **without revealing the data that makes it true**.

Concrete example for BCA Green Mark:

```
Without ZKP:
  Edge device sends → raw EUI readings, chiller COP logs, LPD measurements
  BCA receives      → raw sensor data (privacy risk, attack surface)

With ZKP:
  Edge device sends → proof that "EUI < 115, COP ≥ 0.65, LPD ≤ 15" is satisfied
  BCA receives      → cert_level: "gold", all_criteria_pass: true
                      (raw sensor values never leave the building)
```

The verifier learns only what the prover intended to reveal — the **attestation** — and has cryptographic assurance that the underlying computation was performed correctly.

---

## Why EdgeSentry uses ZKPs

EdgeSentry devices sit at the boundary between private operational data and public compliance reporting. ZKPs let the device prove compliance to regulators without exposing the raw readings that produced it:

| Without ZKP | With ZKP |
|---|---|
| Building operator sends raw energy meter data to BCA | Sends only cert level + pass/fail flags |
| OT asset inventory must be disclosed to prove no rogue software | Proves all hashes ∈ allowlist without revealing the list |
| Vessel AIS track sent to port authority to prove lane compliance | Proves track satisfies routing rules without full position history |

This is the **"verifiable edge computing"** property: the computation happens on a device the operator controls, and the output is verifiable by a third party with zero data exposure.

---

## How it works in this codebase

### 1. The `edgesentry-zkp` crate

Defines the generic interface. No business logic lives here.

```rust
pub trait ZkProgram: Send + Sync {
    fn prove(&self, private_inputs: &[u8]) -> Result<ZkProof, ZkError>;
}

pub struct ZkProof {
    pub framework:     String, // "mock" | "sp1"
    pub program_id:    String, // e.g. "bca-green-mark-2021-v1-mock"
    pub proof_bytes:   String, // base64 — the cryptographic proof
    pub public_values: String, // base64(JSON) — the attestation (what the verifier sees)
}
```

`public_values` is the only thing that crosses the trust boundary. `proof_bytes` is used by the verifier to confirm the computation was run honestly.

### 2. Implementing crates (in clarus)

Each domain provides a concrete `ZkProgram`:

| Crate | Program | Private inputs | Public attestation |
|---|---|---|---|
| `clarus/edge` | `GreenMarkProgram` | `eui_kwh_m2`, `chiller_cop`, `lpd_w_m2` | `cert_level`, `all_criteria_pass`, `cop_pass`, `lpd_pass` |
| `clarus/edge` | `OtIntegrityProgram` | component hash list, allowlist | `all_authorized`, `unauthorized_count`, `status` |

### 3. The proof is stored in the WORM audit chain

When the edge daemon generates a proof, it embeds it in the WORM audit record:

```json
{
  "sequence": 29,
  "rule_id": "EUI_GOLD_EXCEEDED",
  "zk_proof": {
    "framework": "mock",
    "program_id": "bca-green-mark-2021-v1-mock",
    "proof_bytes": "<base64>",
    "public_values": "<base64(GreenMarkAttestation JSON)>"
  }
}
```

The record is hash-chained and Object Lock-protected. Raw sensor data is never stored.

### 4. Consumption (documaris)

`documaris` reads the WORM chain and decodes `public_values`:

```typescript
const att: GreenMarkAttestation = JSON.parse(atob(record.zk_proof.public_values));
// → { cert_level: "gold", all_criteria_pass: true, ... }
// Raw EUI/COP/LPD values: never fetched
```

---

## Proving frameworks

### Mock (current)

The mock framework encodes `private_inputs` directly as `proof_bytes` (base64). It is not cryptographically secure — anyone with the proof can reconstruct the inputs. Used for local development and demos where the trust property is not yet required.

### SP1 by Succinct (production target)

[SP1](https://github.com/succinctlabs/sp1) compiles Rust programs to RISC-V and generates a Groth16 or PLONK proof that can be verified on-chain (EVM) or in-browser (WASM verifier). The guest program runs identically to the native Rust code — no separate circuit DSL required.

Migration path: replace `ZkFramework::Mock` with `ZkFramework::Sp1` in the implementing crate. The `ZkProof` envelope and `public_values` format are unchanged.

---

## Why SP1 is not a dependency of `edgesentry-zkp`

`sp1-sdk` pulls in `alloy-consensus`, `risc0-circuit-*`, and other large crates under LGPL/MPL licences. Adding them to `edgesentry-zkp` would:

1. Force every consumer (including embedded targets) to build them
2. Introduce licence constraints on the Apache 2.0 / MIT `edgesentry-zkp` crate

The SP1 SDK is declared only in the crate that implements `ZkProgram` — e.g. `clarus/edge/Cargo.toml`. `edgesentry-zkp` itself has zero heavyweight dependencies.

---

## Regulatory fit

| Programme | ZkProgram | Regulation |
|---|---|---|
| BCA Green Mark (BEAMP) | `GreenMarkProgram` | BCA Green Mark 2021 — EUI / COP / LPD thresholds |
| PIER71-02 (Cybersecurity) | `OtIntegrityProgram` | IACS UR E26/E27 — OT software integrity |
| PIER71-17 (Emissions) | _planned_ | EU MRV / IMO CII — voyage CO₂ intensity |

---

*See also: [`crates.md`](crates.md) · [clarus ZKP architecture](https://github.com/edgesentry/clarus/blob/main/docs/ref-architecture.md)*
