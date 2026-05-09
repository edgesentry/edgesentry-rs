use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZkError {
    #[error("proof generation failed: {0}")]
    ProveFailed(String),

    #[error("proof verification failed: {0}")]
    VerifyFailed(String),

    #[error("serialisation error: {0}")]
    Serialise(String),

    #[error("unsupported framework: {0}")]
    UnsupportedFramework(String),

    #[error("invalid proof bytes: {0}")]
    InvalidProof(String),
}
