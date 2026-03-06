use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];
pub type Signature64 = [u8; 64];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub device_id: String,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub payload_hash: Hash32,
    #[serde(with = "signature64_serde")]
    pub signature: Signature64,
    pub prev_record_hash: Hash32,
    pub object_ref: String,
}

impl AuditRecord {
    pub fn hash(&self) -> Hash32 {
        let bytes = postcard::to_allocvec(self).expect("AuditRecord serialization should not fail");
        *blake3::hash(&bytes).as_bytes()
    }

    pub fn zero_hash() -> Hash32 {
        [0u8; 32]
    }
}

mod signature64_serde {
    use serde::{de::Error as DeError, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::<u8>::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(D::Error::custom(format!(
                "invalid signature length: expected 64, got {}",
                bytes.len()
            )));
        }

        let mut out = [0u8; 64];
        out.copy_from_slice(&bytes);
        Ok(out)
    }
}
