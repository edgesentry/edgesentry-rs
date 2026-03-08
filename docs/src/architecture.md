# Architecture

## Device Side vs Cloud Side

This system assumes a public-infrastructure IoT deployment where field devices (for example, lift inspection devices) send inspection evidence to cloud services.

### Device side (resource-constrained edge)

The device-side responsibility is implemented by `edgesentry_rs::agent` and related modules.

- Generate inspection event payloads (door check, vibration check, emergency brake check)
- Compute `payload_hash` (BLAKE3)
- Sign the hash using an Ed25519 private key
- Link each event to the previous record hash (`prev_record_hash`) so records form a chain
- Send only compact audit metadata plus object reference (`object_ref`) to keep edge-side cost low

### Cloud side (verification and trust enforcement)

The cloud-side responsibility is implemented by `edgesentry_rs::ingest` and related modules.

- Verify that the device is known (`device_id` -> public key)
- Verify signature validity for each incoming record
- Enforce sequence monotonicity and reject duplicates
- Enforce hash-chain continuity (`prev_record_hash` must match previous record hash)
- Reject tampered, replayed, or reordered data before persistence

### Shared trust logic

All hashing and verification rules live in the same `edgesentry-rs` crate, keeping logic identical across edge and cloud usage.

## Resource-Constrained Device Design

The device-side design is intentionally lightweight so it can be adapted to Cortex-M class environments.

- **Small cryptographic footprint:** records store fixed-size hashes (`[u8; 32]`) and signatures (`[u8; 64]`)
- **Minimal compute path:** hash and sign only; no heavy server-side validation logic on device
- **Compact wire format readiness:** record structure is deterministic and serializable (`serde` + `postcard` support in core)
- **Offload heavy work to cloud:** duplicate detection, sequence policy checks, and full-chain verification are cloud concerns
- **Tamper-evident by construction:** a one-byte modification breaks signature checks or chain continuity

## Concrete Design Flow

1. Device creates event payload `D`.
2. Device computes `H = hash(D)` and signs `H` → signature `S`.
3. Device emits `AuditRecord { device_id, sequence, timestamp_ms, payload_hash=H, signature=S, prev_record_hash, object_ref }`.
4. Cloud verifies signature with registered public key.
5. Cloud verifies sequence and previous-hash link.
6. If any check fails, ingest is rejected; otherwise the record is accepted.

In short, the edge signs facts, and the cloud enforces continuity and authenticity.
