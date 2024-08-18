use crate::pack_io::reading::flags::is_compressed;
use crate::pack_io::reading::read_steps::decompress;
use crate::pack_io::reading::{FileMeta, ReadResult};
use std::cmp::min;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use tracing::warn;

use super::utils::io;
use super::ReadStep;

/// A [`Read`] + [`Seek`] struct for reading the data corresponding to a file contained in an asset pack.
///
/// This does not account for compressed files. The returned reader will still read the compressed
/// data. For most use cases, use [`AssetFileReader`](AssetFileReader) through [`AssetPackReader::get_file_reader`](super::AssetPackReader::get_file_reader).
pub struct DirectFileReader<'r, R>
where
    R: Read + Seek,
{
    pack_file: &'r mut R,
    file_meta: FileMeta,
}

impl<'r, R> DirectFileReader<'r, R>
where
    R: Read + Seek,
{
    /// Create a new [`DirectFileReader`] and seeks to the start of the file in the asset pack.
    ///
    /// # Parameters
    /// - `pack`: A reader of the asset pack file
    /// - `meta`: [`FileMeta`] of the file being read
    ///
    /// # Errors
    /// [`ReadError::IoError`](super::errors::ReadError::IoError) if seeking fails.
    pub fn new(pack: &'r mut R, meta: FileMeta) -> ReadResult<Self> {
        io!(
            pack.seek(SeekFrom::Start(meta.offset)),
            ReadStep::CreateDirectFileReader(meta)
        )?;
        Ok(Self {
            pack_file: pack,
            file_meta: meta,
        })
    }
}

impl<R> Read for DirectFileReader<'_, R>
where
    R: Read + Seek,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pack_file.stream_position()? < self.file_meta.offset {
            // Make sure we never read beyond the start of the file
            warn!("Asset pack seek is outside of the file being read. Clamping to file start.");
            self.pack_file
                .seek(SeekFrom::Start(self.file_meta.offset))?;
        }

        // This should never overflow because we checked for that above
        let seek_from_file_start = self.pack_file.stream_position()? - self.file_meta.offset;
        if seek_from_file_start > self.file_meta.size {
            // Make sure we never read beyond the end of the file
            let end_offset = self.file_meta.offset + self.file_meta.size;
            warn!("Asset pack seek is outside of the file being read. Clamping to file end.");
            self.pack_file.seek(SeekFrom::Start(end_offset))?;
        }

        let seek_from_file_start = self.pack_file.stream_position()? - self.file_meta.offset;

        // This will never overflow because we checked for that above
        // safe casting it to usize is fine here because IF the file size overflows the usize,
        // We will just get a smaller value that won't exceed the real max_read_length.
        let max_read_length = (self.file_meta.size - seek_from_file_start) as usize;

        let read_length = min(buf.len(), max_read_length);
        self.pack_file.read(&mut buf[..read_length])
    }
}

impl<R> Seek for DirectFileReader<'_, R>
where
    R: Read + Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(pos) => {
                let seek_pos = self.file_meta.offset + pos;
                Ok(self.pack_file.seek(SeekFrom::Start(seek_pos))?)
            }
            SeekFrom::End(pos) => {
                let end_pos = self.file_meta.offset + self.file_meta.size;

                // This works as i128 can contain everything u64 can contain
                let seek_pos = end_pos as i128 + pos as i128;

                if seek_pos < self.file_meta.offset as i128 {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "Tried to seek beyond the start of the file.",
                    ));
                }

                // This is safe as the above statement will return Err for any negative value.
                Ok(self.pack_file.seek(SeekFrom::Start(seek_pos as u64))?)
            }
            SeekFrom::Current(pos) => {
                let current_pos = self.pack_file.stream_position()?;

                let seek_pos = current_pos as i128 + pos as i128;

                if seek_pos < self.file_meta.offset as i128 {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "Tried to seek beyond the start of the file.",
                    ));
                }

                Ok(self.pack_file.seek(SeekFrom::Current(pos))?)
            }
        }
    }
}

/// A [`Read`] + [`Seek`] enum used for reading files from asset packs.
///
/// Unlike [`DirectFileReader`], this enum has variants for readers of decompressed files, and normal files.
/// Always use this instead of the [`DirectFileReader`] unless you need access to the compressed
/// data.
pub enum AssetFileReader<'r, R>
where
    R: Read + Seek,
{
    /// The [`DirectFileReader`] for an uncompressed file
    Normal(DirectFileReader<'r, R>),
    /// The [`File`] pointing to the decompressed temporary file
    Decompressed(File),
}

impl<'r, R: Read + Seek> AssetFileReader<'r, R> {
    /// Create a new [`AssetFileReader`] which decompresses a file if it is stored compressed.
    ///
    /// For most use cases, don't use this constructor. Use [`AssetPackReader::get_file_reader`](super::AssetPackReader::get_file_reader) instead.
    ///
    /// # Parameters
    /// - `file_reader`: The direct file reader to read the data directly from the asset pack.
    /// - `file_meta`: The metadata of the file from the table of contents.
    ///
    /// # Errors
    /// Can fail if decompression fails, or if rewinding the temporary decompressed file fails.
    pub fn new(file_reader: DirectFileReader<'r, R>, file_meta: FileMeta) -> ReadResult<Self> {
        if is_compressed(file_meta.flags) {
            let mut temp = io!(decompress(file_reader), ReadStep::DecompressFile(file_meta))?;
            io!(temp.rewind(), ReadStep::DecompressFile(file_meta))?;
            Ok(AssetFileReader::Decompressed(temp))
        } else {
            Ok(AssetFileReader::Normal(file_reader))
        }
    }
}

impl<'r, R: Read + Seek> Read for AssetFileReader<'r, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            AssetFileReader::Normal(r) => r.read(buf),
            AssetFileReader::Decompressed(r) => r.read(buf),
        }
    }
}

impl<'r, R: Read + Seek> Seek for AssetFileReader<'r, R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            AssetFileReader::Normal(s) => s.seek(pos),
            AssetFileReader::Decompressed(s) => s.seek(pos),
        }
    }
}
