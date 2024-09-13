//! Async API for reading asset pack files
//!
//! If you need a blocking API, use the API at [`super`] instead.

mod errors;
mod file_reader;
pub mod flags;
mod iter_dir;
pub mod pack_group;
mod read_steps;
mod utils;

use cfg_if::cfg_if;
pub use errors::*;
pub use file_reader::*;
pub use iter_dir::*;

use futures_lite::io::{AsyncBufRead, AsyncSeek, BufReader};
use futures_lite::{future, AsyncRead, AsyncReadExt};
use lru::LruCache;
use read_steps::{read_toc, validate_files, validate_header, validate_version};

use async_fs::File;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::path::Path;
use tracing::warn;
use utils::{io, read_bytes};

/// Create an instance of this struct to asynchronously read an asset pack.
///
/// # Examples
/// * Read the TOC from the asset pack:
/// ```no_run
/// use smve_asset_pack::pack_io::reading::async_read::AssetPackReader;
///
/// # async_io::block_on(async {
/// let mut pack_reader = AssetPackReader::new_from_path("./path/to/pack.smap").await?;
/// let toc = pack_reader.get_toc();
/// # smve_asset_pack::pack_io::reading::async_read::ReadResult::Ok(()) });
/// ```
///
/// * Read a file from the asset pack:
/// ```no_run
/// use smve_asset_pack::pack_io::reading::async_read::AssetPackReader;
///
/// # async_io::block_on(async {
/// let mut pack_reader = AssetPackReader::new_from_path("./path/to/pack.smap").await?;
/// let file_reader = pack_reader.get_file_reader("path/to/file.txt").await?.unwrap();
/// # smve_asset_pack::pack_io::reading::async_read::ReadResult::Ok(()) });
/// ```
///
/// * Read asset pack from memory:
/// ```no_run
/// use smve_asset_pack::pack_io::reading::async_read::AssetPackReader;
/// use futures_lite::io::Cursor;
///
/// # async_io::block_on(async {
/// let mut pack_reader = AssetPackReader::new(Cursor::new(b"SMAP\x00\x01...")).await?;
/// let file_reader = pack_reader.get_file_reader("pack/to/file.txt").await?.unwrap();
/// # smve_asset_pack::pack_io::reading::async_read::ReadResult::Ok(()) });
/// ```
/// See also [`AssetFileReader`].
pub struct AssetPackReader<R: ConditionalSendAsyncSeekableBufRead> {
    reader: R,
    toc: TOC,
    directories_cache: LruCache<String, DirectoryInfo>,
    version: u16,
}

impl<R: ConditionalSendAsyncSeekableBufRead> Debug for AssetPackReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetPackReader")
            .field("version", &self.version)
            .finish()
    }
}

impl AssetPackReader<BufReader<File>> {
    /// Create a new [`AssetPackReader`] from a path, verifies it, and reads the TOC.
    ///
    /// # Parameters
    /// - `pack_path`: Path to the asset pack file
    ///
    /// # Errors
    /// Will fail if the pack file is invalid or if the version of the format is incompatible.
    ///
    /// See [`ReadError`].
    pub async fn new_from_path(pack_path: impl AsRef<Path>) -> ReadResult<Self> {
        let pack_path = pack_path.as_ref();

        let file = io!(
            File::open(pack_path).await,
            ReadStep::OpenPack(pack_path.to_path_buf())
        )?;

        Self::new_from_read(file).await
    }
}

impl<R: ConditionalSendAsyncReadAndSeek> AssetPackReader<BufReader<R>> {
    /// Creates a new [`AssetPackReader`] from a [`AsyncRead`], verifies it, and reads its TOC.
    ///
    /// **NOTE**: If your read type already implements [`AsyncBufRead`], use [`new`](Self::new) instead
    /// to avoid double buffering.
    ///
    /// # Parameters
    /// - `reader`: A reader containing an asset pack. This will be wrapped in a [`BufReader`].
    ///
    /// # Errors
    /// Will fail if the pack file is invalid or if the version of the format is incompatible.
    ///
    /// See [`ReadError`].
    pub async fn new_from_read(reader: R) -> ReadResult<Self> {
        let buf_reader = BufReader::new(reader);

        Self::new(buf_reader).await
    }
}

impl<R: AsyncReadExt + AsyncBufRead + ConditionalSendAsyncReadAndSeek> AssetPackReader<R> {
    /// Creates a new [`AssetPackReader`] from a [`AsyncBufRead`], verifies it, and reads its pack
    /// front.
    ///
    /// **NOTE**: If your type don't already implement [`AsyncBufRead`], use [`new_from_read`](Self::new_from_read) instead.
    ///
    /// # Parameters
    /// - `reader`: A buffered reader containing an asset pack.
    ///
    /// # Errors
    /// Will fail if the pack file is invalid or if the version of the format is incompatible.
    ///
    /// See [`ReadError`].
    pub async fn new(mut reader: R) -> ReadResult<Self> {
        validate_header(&mut reader).await?;

        let version = validate_version(&mut reader).await?;

        let expected_toc_hash = io!(read_bytes!(reader, 32), ReadStep::ReadTOC)?;

        let (mut normal_files, mut unique_files) =
            read_toc(&mut reader, &expected_toc_hash).await?;

        validate_files(&mut reader, &mut normal_files, &mut unique_files).await?;

        let toc = TOC {
            normal_files,
            unique_files,
        };

        Ok(Self {
            reader,
            toc,
            directories_cache: LruCache::new(NonZeroUsize::new(16).unwrap()),
            version,
        })
    }

    /// Gets the version of the format of the asset pack file.
    pub fn get_version(&self) -> u16 {
        self.version
    }

    /// Returns the TOC of the asset pack.
    ///
    /// # See also
    /// [`TOC`]
    pub fn get_toc(&mut self) -> &TOC {
        &self.toc
    }

    /// Returns a [`AssetFileReader`] for a specified file.
    ///
    /// # Parameters
    /// - `path`: The path of the file to be read relative to the original assets directory (without `./`)
    ///
    /// # Errors
    /// See [`ReadError`].
    ///
    /// # See Also
    /// If you wish to read a pack-unique file, see [`get_unique_file_reader`](Self::get_unique_file_reader)
    pub async fn get_file_reader(
        &mut self,
        path: &str,
    ) -> ReadResult<Option<AssetFileReader<'_, R>>> {
        let toc = &self.get_toc().normal_files;
        let meta = toc.get(path);
        if meta.is_none() {
            return Ok(None);
        }
        let meta = *meta.unwrap();

        let file_reader = DirectFileReader::new(&mut self.reader, meta).await?;

        AssetFileReader::new(file_reader, meta).await.map(Some)
    }

    /// Returns a [`DirectFileReader`] for a specified pack-unique file.
    ///
    /// # Parameters
    /// - `path`: The path of the pack-unique file to be read relative to the `__unique__` directory.
    ///
    /// # Errors
    /// Returns an error if creating the file reader fails.
    /// See [`ReadError`].
    ///
    /// # See Also
    /// If you wish to read an asset not marked as unique, see [`get_file_reader`](Self::get_file_reader).
    pub async fn get_unique_file_reader(
        &mut self,
        path: &str,
    ) -> ReadResult<Option<AssetFileReader<'_, R>>> {
        let unique_files = &self.get_toc().unique_files;
        let meta = unique_files.get(path);
        if meta.is_none() {
            return Ok(None);
        }
        let meta = *meta.unwrap();

        let file_reader = DirectFileReader::new(&mut self.reader, meta).await?;

        AssetFileReader::new(file_reader, meta).await.map(Some)
    }

    /// Checks if a file exists in the asset pack.
    ///
    /// # Parameters
    /// - `path`: The path of the file to check relative to the original assets directory (without `./`)
    pub fn has_file(&mut self, path: &str) -> bool {
        let toc = &self.get_toc().normal_files;
        let meta = toc.get(path);
        meta.is_some()
    }

    /// Returns the flags for a specified file.
    ///
    /// # Parameters
    /// - `path`: The path of the file to be read relative to the original assets directory
    ///   (without `./`)
    ///
    /// # See also
    /// [Flags](https://github.com/smve-rs/smve_asset_pack/blob/master/docs/specification/v1.md#file-flags)
    pub fn get_flags(&mut self, path: &str) -> Option<u8> {
        let toc = &self.get_toc().normal_files;
        let meta = toc.get(path)?;

        Some(meta.flags)
    }

    /// Checks whether a specified path is a directory in the pack file.
    ///
    /// NOTE: If the directory name is not cached (16 directories will be cached in an LRU cache at any one time),
    /// this function will iterate through every file in the TOC and checking if they belong to the directory.
    /// Don't use this unless you absolutely have to.
    ///
    /// # Parameters
    /// - `path`: The path of the directory relative to the assets directory (without ./)
    ///
    /// # Returns
    /// `true` if the path is a directory, `false` if the path is not a directory`
    pub async fn has_directory(&mut self, path: &str) -> bool {
        if !path.ends_with('/') {
            warn!("`has_directory` returned `false` because your path does not end with a trailing slash!");
            return false;
        }

        matches!(
            self.get_directory_info(path).await,
            DirectoryInfo::Directory(_)
        )
    }

    async fn get_directory_info(&mut self, path: &str) -> DirectoryInfo {
        let without_slash = &path[0..path.len() - 1];

        if self.directories_cache.peek(without_slash).is_none() {
            for (index, (file_name, _)) in self.toc.normal_files.iter().enumerate() {
                if file_name.starts_with(path) {
                    self.directories_cache
                        .put(without_slash.to_owned(), DirectoryInfo::Directory(index));
                    return DirectoryInfo::Directory(index);
                }
                // Yield every 1024 operations (including the first one)
                if index % 1024 == 0 {
                    future::yield_now().await;
                }
            }
            self.directories_cache
                .put(without_slash.to_owned(), DirectoryInfo::NotADirectory);
        }

        *self.directories_cache.get(without_slash).unwrap()
    }
}

impl<R: ConditionalSendAsyncSeekableBufRead + 'static> AssetPackReader<R> {
    /// Converts the inner reader of an asset pack to a boxed generic reader.
    pub fn box_reader(self) -> AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>> {
        let boxed_reader = Box::new(self.reader) as Box<dyn ConditionalSendAsyncSeekableBufRead>;

        AssetPackReader {
            reader: boxed_reader,
            toc: self.toc,
            directories_cache: self.directories_cache,
            version: self.version,
        }
    }
}

/// Stores the sections making up the Table of Contents.
pub struct TOC {
    /// The hashmap with the file path as a key and the [`FileMeta`] associated with the path as
    /// the value.
    ///
    /// This does NOT contain pack-unique files.
    pub normal_files: IndexMap<String, FileMeta>,
    /// The hashmap with the path of the pack-unique file (without a leading __unique__/) as a key
    /// and the [`FileMeta`] associated with the path as the value.
    pub unique_files: HashMap<String, FileMeta>,
}

/// The type that is stored in the directory cache.
#[derive(Clone, Copy)]
pub enum DirectoryInfo {
    /// If the requested path does not exist in the pack as a directory.
    NotADirectory,
    /// If the requested path is a directory. It stores the index of the first file in the
    /// directory in the TOC.
    Directory(usize),
}

/// Information about the file stored in the Table of Contents of the asset pack.
///
/// See also: [V1 Specification](https://github.com/smve-rs/asset_pack/blob/master/docs/specification/v1.md#table-of-contents)
#[derive(Debug, Copy, Clone)]
pub struct FileMeta {
    /// A [`Blake3`](blake3::Hasher) hash of the file data.
    pub hash: [u8; 32],
    /// See [File Flags](https://github.com/smve-rs/asset_pack/blob/master/docs/specification/v1.md#file-flags)
    pub flags: u8,
    /// Offset in bytes from the **start of the pack file**.
    ///
    /// # Important
    /// This is different from what is specified in the specification.
    /// In the specification, it stores the offset from **the end of the TOC**,
    /// but this offset is from **the start of the pack file** for easier seeking.
    pub offset: u64,
    /// Size of the file in bytes.
    pub size: u64,
}

cfg_if! {
    if #[cfg(feature = "non_send_readers")] {
        /// A marker trait automatically implemented for anything that implements both [`AsyncBufRead`] and
        /// [`AsyncSeek`] which may be [`Send`] and [`Sync`] depending on the configuration.
        pub trait ConditionalSendAsyncSeekableBufRead:
            AsyncSeek + AsyncBufRead + AsyncRead + Unpin {}

        impl<T: AsyncBufRead + AsyncRead + AsyncSeek + Unpin> ConditionalSendAsyncSeekableBufRead for T {}

        pub trait ConditionalSendAsyncReadAndSeek: AsyncSeek + AsyncRead + Unpin {}

        impl<T: AsyncSeek + AsyncRead + Unpin> ConditionalSendAsyncReadAndSeek for T {}
    } else {
        /// A marker trait automatically implemented for anything that implements both [`AsyncBufRead`] and
        /// [`AsyncSeek`] which may be [`Send`] and [`Sync`] depending on the configuration.
        pub trait ConditionalSendAsyncSeekableBufRead:
            AsyncSeek + AsyncBufRead + AsyncRead + Unpin + Send + Sync {}

        impl<T: AsyncBufRead + AsyncRead + AsyncSeek + Unpin + Send + Sync>
            ConditionalSendAsyncSeekableBufRead for T {}

        /// A marker trait automatically implemented for anything that implements both [`AsyncRead`] and [`AsyncSeek`]
        /// which may be [`Send`] and [`Sync`] depending on the configuration.
        pub trait ConditionalSendAsyncReadAndSeek: AsyncSeek + AsyncRead + Unpin + Send + Sync {}

        impl<T: AsyncSeek + AsyncRead + Unpin + Send + Sync> ConditionalSendAsyncReadAndSeek for T {}
    }
}
