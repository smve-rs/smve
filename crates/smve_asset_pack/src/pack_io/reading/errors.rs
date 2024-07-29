use thiserror::Error;

/// Error raised from reading the asset pack.
#[derive(Error, Debug)]
pub enum ReadError {
    /// IO error from file operations
    #[error("IO Error: {source}")]
    IoError {
        #[from]
        /// The [`std::io::Error`].
        source: std::io::Error,
    },
    /// The pack file does not start with the magic byte sequence.
    #[error("Invalid pack file!")]
    InvalidPackFile,
    /// The pack file is encoded in a version that this version of the library does not support.
    #[error("Version {0} is not supported! This version of the reader only supports version 1 and below.")]
    IncompatibleVersion(u16),
    /// Errors during conversion of the stored file path into a rust UTF-8 string.
    #[error("File path {path:?} could not be converted to UTF-8! {source}")]
    Utf8Error {
        /// The origin error
        source: std::str::Utf8Error,
        /// The exact bytes stored in the file that failed to convert to UTF-8
        path: Box<[u8]>,
    },
    /// The TOC has been modified or damaged.
    #[error("Table of contents does not match the stored hash! This probably means it was damaged or modified.")]
    DamagedTOC,
    /// The Directory List has been modified or damaged.
    #[error("Directory list does not match the stored hash! This probably means it was damaged or modified.")]
    DamagedDirectoryList,
    /// The file data has been modified or damaged.
    #[error("File at {0} does not match its stored hash! This probably means that it was damaged or modified.")]
    DamagedFile(String),
    /// The requested file does not exist in the asset pack.
    #[error("Requested file at {0} does not exist in the pack file!")]
    FileNotFound(String),
    /// The requested pack-unique file does not exist in the asset pack.
    #[error("Requested pack-unique file at {0} does not exist in the pack file!")]
    UniqueFileNotFound(String),
    /// The requested directory does not exist in the asset pack.
    #[error("Requested directory at {0} does not exist in the pack file!")]
    DirectoryNotFound(String),
}

/// Shorthand type for [`Result<T, ReadError>`]
pub type ReadResult<T> = Result<T, ReadError>;
