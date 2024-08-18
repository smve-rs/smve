use std::{fmt::Display, path::PathBuf};

use snafu::{Location, Snafu};

use super::FileMeta;

/// Error raised from reading the asset pack.
#[derive(Snafu, Debug)]
#[snafu(context(suffix(Ctx)), visibility(pub(super)))]
pub enum ReadError {
    /// If a pack group root directory is not a directory.
    #[snafu(display("Specified pack group root directory at {} is not a directory!", path.display()))]
    NotADirectory {
        /// The path to the passed in "root directory".
        path: PathBuf,
    },
    /// IO error from file operations
    #[snafu(display("Encountered IO Error: {source} while {step}. Error occurred at {location}"))]
    IoError {
        /// The [`std::io::Error`].
        source: std::io::Error,
        /// The read step at which the error occurred.
        step: ReadStep,
        /// The source code location where the error occurred.
        #[snafu(implicit)]
        location: Location,
    },
    /// The pack file does not start with the magic byte sequence.
    #[snafu(display("Invalid pack file!"))]
    InvalidPackFile,
    /// The pack file is encoded in a version that this version of the library does not support.
    #[snafu(display("Version {version} is not supported! This version of the reader only supports version 1 and below."))]
    IncompatibleVersion {
        /// The version specified in the pack file.
        version: u16,
    },
    /// Errors during conversion of the stored file path into a rust UTF-8 string.
    #[snafu(display("File path {path:?} could not be converted to UTF-8! {source}"))]
    Utf8Error {
        /// The origin error
        source: std::str::Utf8Error,
        /// The exact bytes stored in the file that failed to convert to UTF-8
        path: Box<[u8]>,
    },
    /// The TOC has been modified or damaged.
    #[snafu(display("Table of contents does not match the stored hash! This probably means it was damaged or modified."))]
    DamagedTOC,
    /// The Directory List has been modified or damaged.
    #[snafu(display("Directory list does not match the stored hash! This probably means it was damaged or modified."))]
    DamagedDirectoryList,
    /// The file data has been modified or damaged.
    #[snafu(display("File at {path} does not match its stored hash! This probably means that it was damaged or modified."))]
    DamagedFile {
        /// The path of the file that has been damaged.
        path: String,
    },
    /// Errors when deserializing packs.toml located in asset pack group directories
    #[snafu(display("Failed to deserialize packs.toml file at root directory {}. This probably means its format is not correct. {source}", path.display()))]
    TomlDeserializeError {
        /// The toml deserialize error
        source: toml::de::Error,
        /// The root directory where the packs.toml file is.
        path: PathBuf,
    },
    /// Errors encountered when recursively reading asset pack directories
    #[snafu(display("Failed to recursively read asset pack directory! {source}"))]
    WalkDirError {
        /// The walkdir error
        source: async_walkdir::Error,
    },
}

#[derive(Debug)]
/// A representation of the read steps.
pub enum ReadStep {
    /// Opening an asset pack. Stores the path of the asset pack.
    OpenPack(PathBuf),
    /// Reading the pack front of the asset pack.
    ReadPackFront,
    /// Validating the asset pack header.
    ValidateHeader,
    /// Reading TOC entry for an asset. Stores the path to the asset file.
    ReadTOC(String),
    /// Reading DL entry for an asset. Stores the path to the directory.
    ReadDirectoryList(String),
    /// Validating file hashes.
    ValidateFiles,
    /// Validate the hash of one file. Stores the path to the file.
    ValidateFile(String),
    /// Creating a direct file reader for an asset. Stores the metadata of the asset.
    CreateDirectFileReader(FileMeta),
    /// Decompressing an asset file. Stores the metadata of the asset.
    DecompressFile(FileMeta),
    /// Reading packs.toml. Stores the root directory where the packs.toml file is located.
    ReadPacksToml(PathBuf),
    /// Opening an asset pack while loading an asset pack group. Stores the path to the pack file
    /// being opened.
    LoadGroupOpenPack(PathBuf),
    /// Writing to packs.toml while loading an asset pack group. Stores the root directory where
    /// packs.toml is located.
    LoadGroupWritePacksToml(PathBuf),
}

impl Display for ReadStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadStep::OpenPack(path) => write!(f, "opening asset pack at {}", path.display()),
            ReadStep::ReadPackFront => write!(f, "reading the pack front of the asset pack"),
            ReadStep::ValidateHeader => write!(f, "validating asset pack header"),
            ReadStep::ReadTOC(path) => {
                write!(f, "reading table of contents entry for asset at {path}")
            }
            ReadStep::ReadDirectoryList(path) => {
                write!(f, "reading directory list entry for directory at {path}")
            }
            ReadStep::ValidateFiles => write!(f, "validating file hashes"),
            ReadStep::ValidateFile(path) => {
                write!(f, "validating file hash for asset file at {path}")
            }
            ReadStep::CreateDirectFileReader(meta) => write!(
                f,
                "creating a direct file reader for asset file with meta {meta:#?}"
            ),
            ReadStep::DecompressFile(meta) => {
                write!(f, "decompressing asset file with meta {meta:#?}")
            }
            ReadStep::ReadPacksToml(root_dir) => write!(
                f,
                "reading the packs.toml file at root directory {}",
                root_dir.display()
            ),
            ReadStep::LoadGroupOpenPack(path) => write!(
                f,
                "opening asset pack at {} when loading pack group",
                path.display()
            ),
            ReadStep::LoadGroupWritePacksToml(root_dir) => write!(
                f,
                "writing packs.toml at root directory {} when loading pack group",
                root_dir.display()
            ),
        }
    }
}

/// Shorthand type for [`Result<T, ReadError>`]
pub type ReadResult<T> = Result<T, ReadError>;
