use snafu::{Location, Snafu};
use std::{fmt::Display, path::PathBuf};

/// Errors raised from compiling asset packs
#[derive(Snafu, Debug)]
#[snafu(context(suffix(Ctx)), visibility(pub(super)))]
pub enum CompileError {
    /// The `asset_dir` passed in is not a directory
    #[snafu(display("Asset directory {} is not a directory!", path.display()))]
    NotADirectory {
        /// The passed in asset "directory"
        path: PathBuf,
    },
    /// The `asset_dir` passed in is empty
    #[snafu(display("Asset directory {} is empty!", path.display()))]
    EmptyDirectory {
        /// The passed in asset directory.
        path: PathBuf,
    },
    /// IO errors from file operations
    #[snafu(display("Encountered IO error: {source} while {step}. Error occured at {location}"))]
    IoError {
        /// The IO error itself (See [`std::io::Error`])
        source: std::io::Error,
        /// The compile step at which the error occured.
        step: CompileStep,
        /// The source code location where the error occured.
        #[snafu(implicit)]
        location: Location,
    },
    /// Errors from `ignore` for recursively reading a directory
    #[snafu(display("walkdir error: {source}"))]
    WalkDirError {
        /// The `ignore` error itself (See [`ignore::Error`])
        source: ignore::Error,
    },
}

#[derive(Debug)]
/// A representation of the compile steps.
pub enum CompileStep {
    /// Validating asset directory. Stores the path to the asset directory.
    ValidateAssetDir(PathBuf),
    /// Writing the header for an asset pack.
    WriteHeader,
    /// Reading an asset file from disk. Stores the path to the asset file.
    ReadAssetFile(PathBuf),
    /// Compressing an asset file. Stores the path to the asset file.
    CompressAsset(PathBuf),
    /// Writing TOC entries of the asset file, and writing the asset file data to a temporary blob.
    /// Stores the path to the asset file.
    PreliminaryWrite(PathBuf),
    /// Writing the Table of Contents.
    WriteTOC,
    /// Writing the directory list.
    WriteDirectoryList,
    /// Copying asset blob from temporary file to the asset pack.
    CopyData,
    /// Write generated hashes of various componenets of the pack.
    WriteHashes,
    /// Opening the file to output the asset pack to.
    OpenOutputFile(PathBuf),
}

impl Display for CompileStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileStep::ValidateAssetDir(path) => {
                write!(f, "validating asset directory at {}", path.display())
            }
            CompileStep::WriteHeader => {
                write!(f, "writing pack header")
            }
            CompileStep::ReadAssetFile(path) => {
                write!(f, "reading asset file at {}", path.display())
            }
            CompileStep::CompressAsset(path) => {
                write!(f, "compressing asset file at {}", path.display())
            }
            CompileStep::PreliminaryWrite(path) => {
                write!(
                    f,
                    "writing TOC entry and temporary file data for asset file at {}",
                    path.display()
                )
            }
            CompileStep::WriteTOC => {
                write!(f, "writing the table of contents")
            }
            CompileStep::WriteDirectoryList => {
                write!(f, "writing the directory list")
            }
            CompileStep::CopyData => {
                write!(f, "copying temporary asset blob into pack file")
            }
            CompileStep::WriteHashes => {
                write!(f, "writing hashes")
            }
            CompileStep::OpenOutputFile(path) => {
                write!(f, "opening output file at {}", path.display())
            }
        }
    }
}

/// Shorthand type for [`Result<T, CompileError>`]
pub type CompileResult<T> = Result<T, CompileError>;
