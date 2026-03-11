# Key Management

This page covers the full lifecycle of Ed25519 device keys used by EdgeSentry-RS:
key generation, secure storage, public key registration, and rotation.

Relevant standards: Singapore CLS-04 / ETSI EN 303 645 §5.4 / JC-STAR STAR-1 R1.2.

---

## 1. Key Generation

Generate a fresh Ed25519 keypair with the `eds` CLI:

```bash
eds keygen
```

Example output:

```json
{
  "private_key_hex": "ddca9848801c658d62a010c4d306d6430a0cdc2c383add1628859258e3acfb93",
  "public_key_hex": "4bb158f302c0ad9261c0acfa95e17144ae7249eb0973bbfaeae4501165887a77"
}
```

Save to a file:

```bash
eds keygen --out device-lift-01.key.json
```

Each device must have a **unique** keypair. Never reuse keys across devices.

---

## 2. Deriving the Public Key from an Existing Private Key

If you already have a `private_key_hex` and need to confirm the matching public key:

```bash
eds inspect-key --private-key-hex <64-hex-char-private-key>
```

Example:

```bash
eds inspect-key \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101
```

Output:

```json
{
  "private_key_hex": "0101010101010101010101010101010101010101010101010101010101010101",
  "public_key_hex": "8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"
}
```

---

## 3. Secure Private Key Storage

The private key must be kept secret on the device. Recommended practices:

| Environment | Recommended storage |
|-------------|---------------------|
| Development / CI | Environment variable (`DEVICE_PRIVATE_KEY_HEX`) — never commit to version control |
| Production (software) | Encrypted secrets store (e.g., HashiCorp Vault, AWS Secrets Manager, Azure Key Vault) |
| Production (hardware) | Hardware Security Module (HSM) or Trusted Execution Environment (TEE) — see [#54](https://github.com/edgesentry/edgesentry-rs/issues/54) for the planned HSM path |

File-based storage (development only):

```bash
chmod 600 device-lift-01.key.json
```

Never expose `private_key_hex` in logs, HTTP responses, or error messages.

---

## 4. Registering the Public Key (Cloud Side)

After generating a keypair, register the device's public key in `IntegrityPolicyGate`
before any records are ingested:

```rust
use edgesentry_rs::{IntegrityPolicyGate, parse_fixed_hex};
use ed25519_dalek::VerifyingKey;

let public_key_bytes = parse_fixed_hex::<32>(&public_key_hex)?;
let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;

let mut gate = IntegrityPolicyGate::new();
gate.register_device("lift-01", verifying_key);
```

The `device_id` string passed to `register_device` must exactly match the
`device_id` field in every `AuditRecord` signed by that device.

Any record from an unknown `device_id` is rejected with `IngestError::UnknownDevice`.

---

## 5. Key Rotation

Rotate a device key when:

- The private key may have been exposed
- The device is being decommissioned and reprovisioned
- Your security policy requires periodic rotation

**Rotation procedure:**

1. Generate a new keypair on or for the new device configuration:
   ```bash
   eds keygen --out device-lift-01-v2.key.json
   ```

2. Register the new public key alongside the old one (the gate allows
   multiple keys per `device_id` is not yet supported — register under a
   new `device_id` such as `lift-01-v2` during the transition window).

3. Update the device to sign new records with the new private key and the
   new `device_id`.

4. Once all in-flight records signed with the old key have been ingested and
   verified, remove the old device registration from the policy gate.

5. Securely delete or revoke the old private key from all storage locations.

> **Note:** Multi-key-per-device support (allowing old and new keys simultaneously
> under the same `device_id`) is tracked in [#57](https://github.com/edgesentry/edgesentry-rs/issues/57).

---

## 6. Software Update Publisher Keys

Software update verification uses a separate set of Ed25519 keys from device signing keys. A **publisher key** belongs to the entity that signs firmware or software packages; a **device signing key** belongs to the individual device that signs audit records. Never mix these roles.

### 6.1 Key generation and storage

Generate a publisher keypair the same way as a device keypair:

```bash
eds keygen --out publisher-acme-firmware.key.json
```

The **private key** must be kept in a high-security offline environment (HSM, air-gapped workstation, or a secrets manager with strict access control). It is used only at build time to sign a release artifact, never on the device itself.

The **public key** is embedded in the device firmware image at manufacture time and loaded into `UpdateVerifier` at runtime:

```rust
use edgesentry_rs::update::UpdateVerifier;
use ed25519_dalek::VerifyingKey;

let public_key_bytes: [u8; 32] = /* bytes baked into firmware */;
let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;

let mut verifier = UpdateVerifier::new();
verifier.register_publisher("acme-firmware", verifying_key);
```

### 6.2 One publisher ID per key

Register each key under a distinct `publisher_id`. Avoid registering the same key under multiple IDs or multiple keys under the same ID unless your threat model explicitly requires it.

```rust
// Correct: one key per publisher
verifier.register_publisher("acme-firmware", firmware_key);
verifier.register_publisher("acme-config",   config_key);

// Avoid: same key shared across publishers — a signature from one
// package type could be accepted for the other
verifier.register_publisher("acme-firmware", shared_key); // ⚠
verifier.register_publisher("acme-config",   shared_key); // ⚠
```

### 6.3 Key confusion attacks

A **key confusion attack** occurs when a signature produced for one package type is submitted as a valid signature for another. `UpdateVerifier` prevents this because:

1. The caller passes an explicit `publisher_id` to `verify()`.
2. The verifier looks up the key registered under that exact ID.
3. A signature by `acme-config`'s key will not verify under `acme-firmware`'s key.

This only holds when each publisher has a unique key. If keys are shared across publishers (see §6.2), the isolation breaks.

### 6.4 Publisher key rotation

Rotate a publisher key when the private key may have been exposed or your security policy requires periodic rotation.

1. Generate a new keypair offline.
2. Sign the next firmware release with the new private key.
3. Distribute a firmware update that embeds the new public key and calls `register_publisher` with the new key. Include both old and new keys during the transition window so devices on either firmware version can verify updates.
4. After all devices have moved to the new firmware, remove the old key registration.
5. Securely destroy the old private key.

### 6.5 FFI (C/C++ devices)

For devices integrating via the C/C++ FFI bridge, publisher key verification will be exposed as `eds_verify_update` (tracked in [#80](https://github.com/edgesentry/edgesentry-rs/issues/80)). Until that function is available, C/C++ devices must call into Rust via a thin wrapper or handle publisher verification at the application layer.

The public key bytes to pass to `eds_verify_update` are the same 32-byte Ed25519 public key described above — provision them into the device at manufacture time, stored in a read-only flash region or secure element.

---

## 7. HSM Path (CLS Level 4)

For CLS Level 4 and high-assurance deployments, private keys should never exist
as extractable byte arrays. Instead, signing operations should be performed inside
an HSM or TEE, with the private key material never leaving the secure boundary.

The planned `edgesentry-bridge` C/C++ FFI layer (#53) and HSM integration (#54)
will provide a signing interface that delegates the Ed25519 `sign` operation to an
HSM-backed provider without exposing the raw key bytes to application code.
