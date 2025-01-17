#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("Worker initialization failed: {0}")]
    InitializationError(String),
    
    #[error("Task processing failed: {0}")]
    ProcessingError(String),
    
    #[error("Worker pool is at capacity: {0}")]
    CapacityError(String),
    
    #[error("Worker state error: {0}")]
    StateError(String),
    
    #[error(transparent)]
    OptimizerError(#[from] crate::utils::OptimizerError),
}

pub type WorkerResult<T> = Result<T, WorkerError>;

impl From<tokio::sync::AcquireError> for WorkerError {
    fn from(err: tokio::sync::AcquireError) -> Self {
        WorkerError::CapacityError(format!("Failed to acquire worker: {}", err))
    }
}

impl From<std::io::Error> for WorkerError {
    fn from(err: std::io::Error) -> Self {
        WorkerError::ProcessingError(format!("IO error during processing: {}", err))
    }
}

impl From<tokio::sync::TryAcquireError> for WorkerError {
    fn from(err: tokio::sync::TryAcquireError) -> Self {
        WorkerError::CapacityError(format!("Failed to acquire worker (try): {}", err))
    }
}

impl<T> From<std::sync::PoisonError<T>> for WorkerError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        WorkerError::StateError("Worker pool state is corrupted".to_string())
    }
}
