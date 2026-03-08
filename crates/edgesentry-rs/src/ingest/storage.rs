use std::collections::HashMap;

use thiserror::Error;

use crate::crypto::compute_payload_hash;
use crate::record::AuditRecord;
use super::policy::IntegrityPolicyGate;
use super::verify::IngestError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestDecision {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationLogEntry {
    pub decision: IngestDecision,
    pub device_id: String,
    pub sequence: u64,
    pub message: String,
}

pub trait RawDataStore {
    type Error: std::error::Error;

    fn put(&mut self, object_ref: &str, payload: &[u8]) -> Result<(), Self::Error>;
}

pub trait AuditLedger {
    type Error: std::error::Error;

    fn append(&mut self, record: AuditRecord) -> Result<(), Self::Error>;
}

pub trait OperationLogStore {
    type Error: std::error::Error;

    fn write(&mut self, entry: OperationLogEntry) -> Result<(), Self::Error>;
}

#[derive(Debug, Error)]
pub enum IngestServiceError {
    #[error("ingest verification failed: {0}")]
    Verify(#[from] IngestError),
    #[error("payload hash mismatch for device={device_id} sequence={sequence}")]
    PayloadHashMismatch { device_id: String, sequence: u64 },
    #[error("raw data store error: {0}")]
    RawDataStore(String),
    #[error("audit ledger error: {0}")]
    AuditLedger(String),
    #[error("operation log error: {0}")]
    OperationLog(String),
}

pub struct IngestService<R, L, O>
where
    R: RawDataStore,
    L: AuditLedger,
    O: OperationLogStore,
{
    policy: IntegrityPolicyGate,
    raw_data_store: R,
    audit_ledger: L,
    operation_log: O,
}

impl<R, L, O> IngestService<R, L, O>
where
    R: RawDataStore,
    L: AuditLedger,
    O: OperationLogStore,
{
    pub fn new(policy: IntegrityPolicyGate, raw_data_store: R, audit_ledger: L, operation_log: O) -> Self {
        Self {
            policy,
            raw_data_store,
            audit_ledger,
            operation_log,
        }
    }

    pub fn register_device(&mut self, device_id: impl Into<String>, key: ed25519_dalek::VerifyingKey) {
        self.policy.register_device(device_id, key);
    }

    pub fn ingest(&mut self, record: AuditRecord, raw_payload: &[u8], cert_identity: Option<&str>) -> Result<(), IngestServiceError> {
        let payload_hash = compute_payload_hash(raw_payload);
        if payload_hash != record.payload_hash {
            self.log_rejection(&record, "payload hash mismatch");
            return Err(IngestServiceError::PayloadHashMismatch {
                device_id: record.device_id,
                sequence: record.sequence,
            });
        }

        if let Err(error) = self.policy.enforce(&record, cert_identity) {
            self.log_rejection(&record, &error.to_string());
            return Err(IngestServiceError::Verify(error));
        }

        self.raw_data_store
            .put(&record.object_ref, raw_payload)
            .map_err(|e| IngestServiceError::RawDataStore(e.to_string()))?;

        self.audit_ledger
            .append(record.clone())
            .map_err(|e| IngestServiceError::AuditLedger(e.to_string()))?;

        self.operation_log
            .write(OperationLogEntry {
                decision: IngestDecision::Accepted,
                device_id: record.device_id,
                sequence: record.sequence,
                message: "ingest accepted".to_string(),
            })
            .map_err(|e| IngestServiceError::OperationLog(e.to_string()))?;

        Ok(())
    }

    pub fn raw_data_store(&self) -> &R {
        &self.raw_data_store
    }

    pub fn audit_ledger(&self) -> &L {
        &self.audit_ledger
    }

    pub fn operation_log(&self) -> &O {
        &self.operation_log
    }

    fn log_rejection(&mut self, record: &AuditRecord, reason: &str) {
        let _ = self.operation_log.write(OperationLogEntry {
            decision: IngestDecision::Rejected,
            device_id: record.device_id.clone(),
            sequence: record.sequence,
            message: reason.to_string(),
        });
    }
}

#[derive(Debug, Error)]
#[error("in-memory store error: {message}")]
pub struct InMemoryStoreError {
    message: String,
}

#[derive(Default)]
pub struct InMemoryRawDataStore {
    objects: HashMap<String, Vec<u8>>,
}

impl InMemoryRawDataStore {
    pub fn get(&self, object_ref: &str) -> Option<&[u8]> {
        self.objects.get(object_ref).map(Vec::as_slice)
    }
}

impl RawDataStore for InMemoryRawDataStore {
    type Error = InMemoryStoreError;

    fn put(&mut self, object_ref: &str, payload: &[u8]) -> Result<(), Self::Error> {
        self.objects.insert(object_ref.to_string(), payload.to_vec());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryAuditLedger {
    records: Vec<AuditRecord>,
}

impl InMemoryAuditLedger {
    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }
}

impl AuditLedger for InMemoryAuditLedger {
    type Error = InMemoryStoreError;

    fn append(&mut self, record: AuditRecord) -> Result<(), Self::Error> {
        self.records.push(record);
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryOperationLog {
    entries: Vec<OperationLogEntry>,
}

impl InMemoryOperationLog {
    pub fn entries(&self) -> &[OperationLogEntry] {
        &self.entries
    }
}

impl OperationLogStore for InMemoryOperationLog {
    type Error = InMemoryStoreError;

    fn write(&mut self, entry: OperationLogEntry) -> Result<(), Self::Error> {
        self.entries.push(entry);
        Ok(())
    }
}

#[cfg(feature = "s3")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3Backend {
    AwsS3,
    Minio,
}

#[cfg(feature = "s3")]
#[derive(Debug, Clone)]
pub struct S3ObjectStoreConfig {
    pub backend: S3Backend,
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

#[cfg(feature = "s3")]
impl S3ObjectStoreConfig {
    pub fn for_aws_s3(bucket: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            backend: S3Backend::AwsS3,
            bucket: bucket.into(),
            region: region.into(),
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
        }
    }

    pub fn for_minio(
        bucket: impl Into<String>,
        region: impl Into<String>,
        endpoint: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        Self {
            backend: S3Backend::Minio,
            bucket: bucket.into(),
            region: region.into(),
            endpoint: Some(endpoint.into()),
            access_key_id: Some(access_key_id.into()),
            secret_access_key: Some(secret_access_key.into()),
        }
    }
}

#[cfg(feature = "s3")]
#[derive(Debug, Error)]
pub enum S3StoreError {
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("runtime initialization failed: {0}")]
    Runtime(String),
    #[error("s3 put failed: {0}")]
    Put(String),
}

#[cfg(feature = "s3")]
pub struct S3CompatibleRawDataStore {
    runtime: tokio::runtime::Runtime,
    client: aws_sdk_s3::Client,
    bucket: String,
}

#[cfg(feature = "s3")]
impl S3CompatibleRawDataStore {
    pub fn new(config: S3ObjectStoreConfig) -> Result<Self, S3StoreError> {
        use aws_config::BehaviorVersion;
        use aws_config::Region;
        use aws_credential_types::Credentials;

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| S3StoreError::Runtime(e.to_string()))?;

        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()));

        match config.backend {
            S3Backend::AwsS3 => {
                if let (Some(access_key_id), Some(secret_access_key)) =
                    (config.access_key_id.clone(), config.secret_access_key.clone())
                {
                    let creds = Credentials::new(access_key_id, secret_access_key, None, None, "static");
                    loader = loader.credentials_provider(creds);
                }
            }
            S3Backend::Minio => {
                let endpoint = config.endpoint.clone().ok_or_else(|| {
                    S3StoreError::InvalidConfig("endpoint is required for MinIO backend".to_string())
                })?;
                let access_key_id = config.access_key_id.clone().ok_or_else(|| {
                    S3StoreError::InvalidConfig("access_key_id is required for MinIO backend".to_string())
                })?;
                let secret_access_key = config.secret_access_key.clone().ok_or_else(|| {
                    S3StoreError::InvalidConfig(
                        "secret_access_key is required for MinIO backend".to_string(),
                    )
                })?;

                let creds = Credentials::new(access_key_id, secret_access_key, None, None, "static");
                loader = loader.endpoint_url(endpoint).credentials_provider(creds);
            }
        }

        let shared = runtime.block_on(loader.load());
        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&shared);

        if config.backend == S3Backend::Minio {
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        let client = aws_sdk_s3::Client::from_conf(s3_config_builder.build());

        Ok(Self {
            runtime,
            client,
            bucket: config.bucket,
        })
    }
}

#[cfg(feature = "s3")]
impl RawDataStore for S3CompatibleRawDataStore {
    type Error = S3StoreError;

    fn put(&mut self, object_ref: &str, payload: &[u8]) -> Result<(), Self::Error> {
        let stream = aws_sdk_s3::primitives::ByteStream::from(payload.to_vec());
        self.runtime
            .block_on(
                self.client
                    .put_object()
                    .bucket(&self.bucket)
                    .key(object_ref)
                    .body(stream)
                    .send(),
            )
            .map_err(|e| S3StoreError::Put(e.to_string()))?;
        Ok(())
    }
}
