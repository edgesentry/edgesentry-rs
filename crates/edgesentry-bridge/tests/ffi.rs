use std::ffi::CString;

use edgesentry_bridge::{
    eds_keygen, eds_record_hash, eds_sign_record, eds_verify_chain, eds_verify_record,
    EdsAuditRecord, EDS_ERR_NULL_PTR, EDS_ERR_STRING_TOO_LONG, EDS_ERR_CHAIN_INVALID, EDS_OK,
};

fn zeroed_record() -> EdsAuditRecord {
    EdsAuditRecord {
        sequence: 0,
        timestamp_ms: 0,
        payload_hash: [0u8; 32],
        signature: [0u8; 64],
        prev_record_hash: [0u8; 32],
        device_id: [0u8; 256],
        object_ref: [0u8; 512],
    }
}

unsafe fn make_keypair() -> ([u8; 32], [u8; 32]) {
    let mut priv_key = [0u8; 32];
    let mut pub_key = [0u8; 32];
    assert_eq!(eds_keygen(priv_key.as_mut_ptr(), pub_key.as_mut_ptr()), EDS_OK);
    (priv_key, pub_key)
}

// ── eds_keygen ────────────────────────────────────────────────────────────────

#[test]
fn keygen_null_private_key_returns_error() {
    let mut pub_key = [0u8; 32];
    let rc = unsafe { eds_keygen(std::ptr::null_mut(), pub_key.as_mut_ptr()) };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn keygen_null_public_key_returns_error() {
    let mut priv_key = [0u8; 32];
    let rc = unsafe { eds_keygen(priv_key.as_mut_ptr(), std::ptr::null_mut()) };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn keygen_produces_nonzero_keys() {
    let mut priv_key = [0u8; 32];
    let mut pub_key = [0u8; 32];
    let rc = unsafe { eds_keygen(priv_key.as_mut_ptr(), pub_key.as_mut_ptr()) };
    assert_eq!(rc, EDS_OK);
    assert_ne!(priv_key, [0u8; 32]);
    assert_ne!(pub_key, [0u8; 32]);
}

#[test]
fn keygen_produces_distinct_keys_each_call() {
    let (priv1, pub1) = unsafe { make_keypair() };
    let (priv2, pub2) = unsafe { make_keypair() };
    assert_ne!(priv1, priv2);
    assert_ne!(pub1, pub2);
}

// ── eds_sign_record ───────────────────────────────────────────────────────────

#[test]
fn sign_record_null_device_id_returns_error() {
    let mut rec = zeroed_record();
    let obj = CString::new("obj").unwrap();
    let priv_key = [1u8; 32];
    let rc = unsafe {
        eds_sign_record(
            std::ptr::null(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        )
    };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn sign_record_null_payload_returns_error() {
    let mut rec = zeroed_record();
    let dev = CString::new("dev").unwrap();
    let obj = CString::new("obj").unwrap();
    let priv_key = [1u8; 32];
    let rc = unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        )
    };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn sign_record_null_out_returns_error() {
    let dev = CString::new("dev").unwrap();
    let obj = CString::new("obj").unwrap();
    let priv_key = [1u8; 32];
    let rc = unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn sign_record_device_id_too_long_returns_error() {
    let mut rec = zeroed_record();
    let long_id = "x".repeat(256); // 256 chars + NUL needs 257 bytes; buf is 256
    let dev = CString::new(long_id).unwrap();
    let obj = CString::new("obj").unwrap();
    let priv_key = [1u8; 32];
    let rc = unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        )
    };
    assert_eq!(rc, EDS_ERR_STRING_TOO_LONG);
}

#[test]
fn sign_record_object_ref_too_long_returns_error() {
    let mut rec = zeroed_record();
    let long_ref = "x".repeat(512); // 512 chars + NUL needs 513 bytes; buf is 512
    let dev = CString::new("dev").unwrap();
    let obj = CString::new(long_ref).unwrap();
    let priv_key = [1u8; 32];
    let rc = unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        )
    };
    assert_eq!(rc, EDS_ERR_STRING_TOO_LONG);
}

#[test]
fn sign_record_fills_fields_correctly() {
    let (priv_key, _) = unsafe { make_keypair() };
    let mut rec = zeroed_record();
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("sensor-01/check-1.bin").unwrap();
    let payload = b"temperature=25.3";
    let rc = unsafe {
        eds_sign_record(
            dev.as_ptr(),
            7,
            1700000000000,
            payload.as_ptr(),
            payload.len(),
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        )
    };
    assert_eq!(rc, EDS_OK);
    assert_eq!(rec.sequence, 7);
    assert_eq!(rec.timestamp_ms, 1700000000000);
    assert_eq!(rec.prev_record_hash, [0u8; 32]);
    assert_ne!(rec.payload_hash, [0u8; 32]);
    assert_ne!(rec.signature, [0u8; 64]);
}

// ── eds_verify_record ─────────────────────────────────────────────────────────

#[test]
fn verify_record_null_record_returns_error() {
    let pub_key = [0u8; 32];
    let rc = unsafe { eds_verify_record(std::ptr::null(), pub_key.as_ptr()) };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn verify_record_null_public_key_returns_error() {
    let rec = zeroed_record();
    let rc = unsafe { eds_verify_record(&rec, std::ptr::null()) };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn verify_record_valid_signature_returns_one() {
    let (priv_key, pub_key) = unsafe { make_keypair() };
    let mut rec = zeroed_record();
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("obj").unwrap();
    unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        );
    }
    let rc = unsafe { eds_verify_record(&rec, pub_key.as_ptr()) };
    assert_eq!(rc, 1);
}

#[test]
fn verify_record_tampered_signature_returns_zero() {
    let (priv_key, pub_key) = unsafe { make_keypair() };
    let mut rec = zeroed_record();
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("obj").unwrap();
    unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        );
    }
    rec.signature[0] ^= 0x01;
    let rc = unsafe { eds_verify_record(&rec, pub_key.as_ptr()) };
    assert_eq!(rc, 0);
}

#[test]
fn verify_record_wrong_public_key_returns_zero() {
    let (priv_key, _) = unsafe { make_keypair() };
    let (_, wrong_pub_key) = unsafe { make_keypair() };
    let mut rec = zeroed_record();
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("obj").unwrap();
    unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        );
    }
    let rc = unsafe { eds_verify_record(&rec, wrong_pub_key.as_ptr()) };
    assert_eq!(rc, 0);
}

// ── eds_record_hash ───────────────────────────────────────────────────────────

#[test]
fn record_hash_null_record_returns_error() {
    let mut hash = [0u8; 32];
    let rc = unsafe { eds_record_hash(std::ptr::null(), hash.as_mut_ptr()) };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn record_hash_null_out_returns_error() {
    let rec = zeroed_record();
    let rc = unsafe { eds_record_hash(&rec, std::ptr::null_mut()) };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn record_hash_is_deterministic() {
    let (priv_key, _) = unsafe { make_keypair() };
    let mut rec = zeroed_record();
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("obj").unwrap();
    unsafe {
        eds_sign_record(
            dev.as_ptr(),
            1,
            0,
            b"payload".as_ptr(),
            7,
            std::ptr::null(),
            obj.as_ptr(),
            priv_key.as_ptr(),
            &mut rec,
        );
    }
    let mut hash1 = [0u8; 32];
    let mut hash2 = [0u8; 32];
    unsafe {
        assert_eq!(eds_record_hash(&rec, hash1.as_mut_ptr()), EDS_OK);
        assert_eq!(eds_record_hash(&rec, hash2.as_mut_ptr()), EDS_OK);
    }
    assert_eq!(hash1, hash2);
    assert_ne!(hash1, [0u8; 32]);
}

#[test]
fn record_hash_changes_when_record_changes() {
    let (priv_key, _) = unsafe { make_keypair() };
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("obj").unwrap();
    let mut rec1 = zeroed_record();
    let mut rec2 = zeroed_record();
    unsafe {
        eds_sign_record(dev.as_ptr(), 1, 0, b"aaa".as_ptr(), 3, std::ptr::null(), obj.as_ptr(), priv_key.as_ptr(), &mut rec1);
        eds_sign_record(dev.as_ptr(), 2, 0, b"bbb".as_ptr(), 3, std::ptr::null(), obj.as_ptr(), priv_key.as_ptr(), &mut rec2);
    }
    let mut hash1 = [0u8; 32];
    let mut hash2 = [0u8; 32];
    unsafe {
        eds_record_hash(&rec1, hash1.as_mut_ptr());
        eds_record_hash(&rec2, hash2.as_mut_ptr());
    }
    assert_ne!(hash1, hash2);
}

// ── eds_verify_chain ──────────────────────────────────────────────────────────

#[test]
fn verify_chain_empty_returns_ok() {
    let rc = unsafe { eds_verify_chain(std::ptr::null(), 0) };
    assert_eq!(rc, EDS_OK);
}

#[test]
fn verify_chain_null_ptr_with_nonzero_count_returns_error() {
    let rc = unsafe { eds_verify_chain(std::ptr::null(), 1) };
    assert_eq!(rc, EDS_ERR_NULL_PTR);
}

#[test]
fn verify_chain_valid_three_records_returns_ok() {
    let (priv_key, _) = unsafe { make_keypair() };
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("sensor-01/check.bin").unwrap();
    let payloads: &[&[u8]] = &[b"check=door", b"check=vibration", b"check=brake"];

    let mut records = [zeroed_record(), zeroed_record(), zeroed_record()];
    let mut prev_hash = [0u8; 32];

    for (i, payload) in payloads.iter().enumerate() {
        let prev_ptr = if i == 0 { std::ptr::null() } else { prev_hash.as_ptr() };
        let rc = unsafe {
            eds_sign_record(
                dev.as_ptr(),
                (i as u64) + 1,
                1700000000000 + i as u64,
                payload.as_ptr(),
                payload.len(),
                prev_ptr,
                obj.as_ptr(),
                priv_key.as_ptr(),
                &mut records[i],
            )
        };
        assert_eq!(rc, EDS_OK);
        unsafe { eds_record_hash(&records[i], prev_hash.as_mut_ptr()) };
    }

    let rc = unsafe { eds_verify_chain(records.as_ptr(), records.len()) };
    assert_eq!(rc, EDS_OK);
}

#[test]
fn verify_chain_tampered_record_returns_chain_invalid() {
    let (priv_key, _) = unsafe { make_keypair() };
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("obj").unwrap();
    let mut records = [zeroed_record(), zeroed_record()];
    let mut prev_hash = [0u8; 32];

    unsafe {
        eds_sign_record(dev.as_ptr(), 1, 0, b"aaa".as_ptr(), 3, std::ptr::null(), obj.as_ptr(), priv_key.as_ptr(), &mut records[0]);
        eds_record_hash(&records[0], prev_hash.as_mut_ptr());
        eds_sign_record(dev.as_ptr(), 2, 1, b"bbb".as_ptr(), 3, prev_hash.as_ptr(), obj.as_ptr(), priv_key.as_ptr(), &mut records[1]);
    }

    // Tamper record[0] so record[1]'s prev_record_hash no longer matches
    records[0].payload_hash[0] ^= 0x01;

    let rc = unsafe { eds_verify_chain(records.as_ptr(), 2) };
    assert_eq!(rc, EDS_ERR_CHAIN_INVALID);
}

#[test]
fn verify_chain_wrong_prev_hash_returns_chain_invalid() {
    let (priv_key, _) = unsafe { make_keypair() };
    let dev = CString::new("sensor-01").unwrap();
    let obj = CString::new("obj").unwrap();
    let mut records = [zeroed_record(), zeroed_record()];
    let mut prev_hash = [0u8; 32];

    unsafe {
        eds_sign_record(dev.as_ptr(), 1, 0, b"aaa".as_ptr(), 3, std::ptr::null(), obj.as_ptr(), priv_key.as_ptr(), &mut records[0]);
        eds_record_hash(&records[0], prev_hash.as_mut_ptr());
        eds_sign_record(dev.as_ptr(), 2, 1, b"bbb".as_ptr(), 3, prev_hash.as_ptr(), obj.as_ptr(), priv_key.as_ptr(), &mut records[1]);
    }

    // Directly corrupt prev_record_hash of record[1]
    records[1].prev_record_hash[0] ^= 0x01;

    let rc = unsafe { eds_verify_chain(records.as_ptr(), 2) };
    assert_eq!(rc, EDS_ERR_CHAIN_INVALID);
}
