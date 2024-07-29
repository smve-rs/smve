use std::path::PathBuf;
use thiserror::Error;

/// Errors raised from compiling asset packs
#[derive(Error, Debug)]
pub enum CompileError {
    /// The `asset_dir` passed in is not a directory
    #[error("{} is not a directory!", .0.display())]
    NotADirectory(PathBuf),
    /// The `asset_dir` passed in is empty
    #[error("Asset directory {} is empty!", .0.display())]
    EmptyDirectory(PathBuf),
    /// IO errors from file operations
    #[error("IO error: {source}")]
    IoError {
        #[from]
        /// The IO error itself (See [`std::io::Error`])
        source: std::io::Error,
    },
    /// Errors from `ignore` for recursively reading a directory
    #[error("walkdir error: {source}")]
    WalkDirError {
        #[from]
        /// The `ignore` error itself (See [`ignore::Error`])
        source: ignore::Error,
    },
}

/// Shorthand type for [`Result<T, CompileError>`]
pub type CompileResult<T> = Result<T, CompileError>;
