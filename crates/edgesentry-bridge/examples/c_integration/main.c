/*
 * C integration example for edgesentry-bridge.
 *
 * Demonstrates:
 *   1. Keypair generation
 *   2. Signing a single audit record
 *   3. Signature verification (valid and tampered)
 *   4. Building and verifying a 3-record hash chain
 *   5. Tamper detection on a chain
 *
 * Build (after `cargo build -p edgesentry-bridge --release`):
 *
 *   macOS:
 *     cc -o c_integration_test main.c \
 *        -I../../include \
 *        -L../../../../target/release \
 *        -ledgesentry_bridge \
 *        -framework Security -framework CoreFoundation \
 *        && ./c_integration_test
 *
 *   Linux:
 *     cc -o c_integration_test main.c \
 *        -I../../include \
 *        -L../../../../target/release \
 *        -ledgesentry_bridge \
 *        -lpthread -ldl \
 *        && ./c_integration_test
 *
 * Or use the provided Makefile: make && ./c_integration_test
 */

#include <stdio.h>
#include <string.h>
#include <assert.h>

#include "edgesentry_bridge.h"

/* Print a byte array as hex. */
static void print_hex(const char *label, const uint8_t *buf, size_t len) {
    printf("  %s: ", label);
    for (size_t i = 0; i < len && i < 8; i++) {
        printf("%02x", buf[i]);
    }
    printf("...  (%zu bytes)\n", len);
}

int main(void) {
    int rc;
    printf("=== edgesentry-bridge C integration test ===\n\n");

    /* ── 1. Keypair generation ──────────────────────────────────────────── */
    uint8_t private_key[32];
    uint8_t public_key[32];

    rc = eds_keygen(private_key, public_key);
    assert(rc == EDS_OK && "eds_keygen must succeed");
    printf("[1] Keypair generated.\n");
    print_hex("public_key", public_key, 32);

    /* ── 2. Sign a single record ────────────────────────────────────────── */
    const char    *payload    = "check=door,status=ok,open_close_cycle=3";
    const uint8_t *zero_hash  = NULL; /* NULL means first record — zero hash */
    EdsAuditRecord record;
    memset(&record, 0, sizeof(record));

    rc = eds_sign_record(
        "lift-01",
        1,                    /* sequence */
        1700000000000ULL,     /* timestamp_ms */
        (const uint8_t *)payload,
        strlen(payload),
        zero_hash,
        "lift-01/check-1.bin",
        private_key,
        &record
    );
    assert(rc == EDS_OK && "eds_sign_record must succeed");
    printf("\n[2] Record signed.\n");
    printf("  device_id : %s\n", (char *)record.device_id);
    printf("  sequence  : %llu\n", (unsigned long long)record.sequence);
    print_hex("payload_hash", record.payload_hash, 32);
    print_hex("signature",    record.signature,    64);

    /* ── 3. Signature verification ──────────────────────────────────────── */
    rc = eds_verify_record(&record, public_key);
    assert(rc == 1 && "signature must be valid");
    printf("\n[3] Signature verification: VALID (expected)\n");

    /* Tamper with one payload_hash byte and confirm rejection. */
    record.payload_hash[0] ^= 0x01;
    rc = eds_verify_record(&record, public_key);
    assert(rc == 0 && "tampered record must be invalid");
    printf("[3] Tampered record     : INVALID (expected)\n");
    record.payload_hash[0] ^= 0x01; /* restore */

    /* ── 4. Build a 3-record hash chain ────────────────────────────────── */
    printf("\n[4] Building 3-record hash chain...\n");

    const char *payloads[3] = {
        "check=door,status=ok,open_close_cycle=3",
        "check=vibration,status=ok,rms=0.18",
        "check=emergency_brake,status=ok,response_ms=120",
    };
    const char *refs[3] = {
        "lift-01/inspection-1.bin",
        "lift-01/inspection-2.bin",
        "lift-01/inspection-3.bin",
    };

    EdsAuditRecord chain[3];
    memset(chain, 0, sizeof(chain));
    uint8_t prev_hash[32];
    memset(prev_hash, 0, sizeof(prev_hash)); /* zero hash for first record */

    for (int i = 0; i < 3; i++) {
        rc = eds_sign_record(
            "lift-01",
            (uint64_t)(i + 1),
            1700000000000ULL + (uint64_t)i * 60000,
            (const uint8_t *)payloads[i],
            strlen(payloads[i]),
            prev_hash,
            refs[i],
            private_key,
            &chain[i]
        );
        assert(rc == EDS_OK && "chain record signing must succeed");

        /* Compute this record's hash for use as next record's prev_record_hash. */
        rc = eds_record_hash(&chain[i], prev_hash);
        assert(rc == EDS_OK && "eds_record_hash must succeed");

        printf("  [%d] signed  sequence=%llu\n",
               i + 1, (unsigned long long)chain[i].sequence);
    }

    /* ── 5. Verify the chain ────────────────────────────────────────────── */
    rc = eds_verify_chain(chain, 3);
    assert(rc == EDS_OK && "chain must be valid");
    printf("\n[5] Chain verification: VALID (expected)\n");

    /* Tamper with the middle record's payload_hash and re-verify. */
    chain[1].payload_hash[0] ^= 0x01;
    rc = eds_verify_chain(chain, 3);
    assert(rc == EDS_ERR_CHAIN_INVALID && "tampered chain must be invalid");
    printf("[5] Tampered chain    : INVALID (expected, rc=%d)\n", rc);

    printf("\n=== All assertions passed. ===\n");
    return 0;
}
