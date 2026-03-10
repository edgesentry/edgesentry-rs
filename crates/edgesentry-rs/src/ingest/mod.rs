mod network_policy;
mod policy;
mod storage;
mod verify;

pub use network_policy::{AllowedSource, NetworkPolicy, NetworkPolicyError};
pub use policy::IntegrityPolicyGate;
pub use storage::{
    AuditLedger, InMemoryAuditLedger, InMemoryOperationLog, InMemoryRawDataStore, IngestDecision,
    IngestService, IngestServiceError, OperationLogEntry, OperationLogStore, RawDataStore,
};
#[cfg(feature = "s3")]
pub use storage::{S3Backend, S3CompatibleRawDataStore, S3ObjectStoreConfig, S3StoreError};
#[cfg(feature = "postgres")]
pub use storage::{PostgresAuditLedger, PostgresOperationLog, PostgresStoreError};
pub use verify::{IngestError, IngestState};
