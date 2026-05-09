use crate::{ZkError, ZkFramework, ZkProof, ZkProgram, verify};

/// Minimal mock ZkProgram for testing the trait and verify() path.
struct MockProgram;

impl ZkProgram for MockProgram {
    fn program_id(&self) -> &str {
        "mock-program-id-v1"
    }

    fn prove(&self, private_inputs: &[u8]) -> Result<ZkProof, ZkError> {
        // Mock: XOR-fold the inputs into a 1-byte "result" and commit it.
        let result: u8 = private_inputs.iter().fold(0u8, |acc, b| acc ^ b);
        Ok(ZkProof {
            framework: ZkFramework::Mock,
            program_id: self.program_id().to_string(),
            proof_bytes: ZkProof::encode(b"mock-proof"),
            public_values: ZkProof::encode(&[result]),
        })
    }
}

#[test]
fn mock_prove_verify_round_trip() {
    let program = MockProgram;
    let inputs = b"sensor-reading-42";

    let proof = program.prove(inputs).expect("prove should succeed");

    assert_eq!(proof.framework, ZkFramework::Mock);
    assert_eq!(proof.program_id, "mock-program-id-v1");

    // Verify round-trip
    let ok = verify(&proof).expect("verify should not error on mock");
    assert!(ok, "mock proof should verify");
}

#[test]
fn mock_public_values_are_deterministic() {
    let program = MockProgram;
    let inputs = b"hello";

    let proof1 = program.prove(inputs).unwrap();
    let proof2 = program.prove(inputs).unwrap();

    assert_eq!(proof1.public_values, proof2.public_values);
}

#[test]
fn tampered_proof_bytes_fail_verification() {
    let program = MockProgram;
    let mut proof = program.prove(b"data").unwrap();
    // Tamper with the proof bytes
    proof.proof_bytes = ZkProof::encode(b"tampered");

    let ok = verify(&proof).expect("verify returns Ok even on invalid proof");
    assert!(!ok, "tampered proof should not verify");
}

#[test]
fn unsupported_framework_returns_error() {
    let proof = ZkProof {
        framework: ZkFramework::Sp1,
        program_id: "some-vkey".to_string(),
        proof_bytes: ZkProof::encode(b"bytes"),
        public_values: ZkProof::encode(b"values"),
    };

    // Without the sp1-verifier feature, verify() should return UnsupportedFramework
    let result = verify(&proof);
    assert!(
        matches!(result, Err(ZkError::UnsupportedFramework(_))),
        "expected UnsupportedFramework, got: {result:?}"
    );
}

#[test]
fn zk_proof_encode_decode_round_trips() {
    let original = b"arbitrary bytes 1234";
    let encoded = ZkProof::encode(original);
    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &encoded,
    )
    .unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn zk_framework_as_str() {
    assert_eq!(ZkFramework::Sp1.as_str(), "sp1");
    assert_eq!(ZkFramework::RiscZero.as_str(), "risc0");
    assert_eq!(ZkFramework::Mock.as_str(), "mock");
}

#[test]
fn zk_proof_serialises_to_json() {
    let proof = ZkProof {
        framework: ZkFramework::Mock,
        program_id: "test-id".to_string(),
        proof_bytes: ZkProof::encode(b"mock-proof"),
        public_values: ZkProof::encode(b"output"),
    };

    let json = serde_json::to_string(&proof).unwrap();
    let back: ZkProof = serde_json::from_str(&json).unwrap();
    assert_eq!(proof, back);
}
