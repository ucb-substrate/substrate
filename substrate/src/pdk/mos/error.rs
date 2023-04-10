#[derive(Debug, thiserror::Error)]
pub enum MosError {
    #[error("mismatched lengths (not all devices have the same channel length)")]
    MismatchedLengths,
    #[error("mismatched number of fingers (not all devices have the same number of fingers)")]
    MismatchedFingers,
    #[error("invalid number of fingers: {0}")]
    InvalidNumFingers(u64),
    #[error("invalid params: {0}")]
    BadParams(String),
    #[error("no devices to draw")]
    NoDevices,
}

pub type MosResult<T> = std::result::Result<T, MosError>;
