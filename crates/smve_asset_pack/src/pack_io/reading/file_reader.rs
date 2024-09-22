use crate::pack_io::common::Flags;
use crate::pack_io::reading::read_steps::decompress;
use crate::pack_io::reading::{FileMeta, ReadResult, ReadStep};
use async_compat::Compat;
use async_tempfile::TempFile;
use futures_lite::{AsyncRead, AsyncSeek, AsyncSeekExt};
use std::cmp::min;
use std::io::{ErrorKind, SeekFrom};
use std::pin::Pin;
use std::task::Poll;

use super::utils::io;

/// A [`AsyncRead`] + [`AsyncSeek`] struct for reading the data corresponding to a file contained in an asset pack.
///
/// This does not account for compressed files. The returned reader will still read the compressed
/// data. For most use cases, use [`AssetFileReader`](AssetFileReader) through [`AssetPackReader::get_file_reader`](super::AssetPackReader::get_file_reader).
pub struct DirectFileReader<'r, R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    pack_file: &'r mut R,
    file_meta: FileMeta,
    seek_pos: u64,
}

impl<'r, R> DirectFileReader<'r, R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    /// Create a new [`DirectFileReader`] and seeks to the start of the file in the asset pack.
    ///
    /// # Parameters
    /// - `pack`: A reader of the asset pack file
    /// - `meta`: [`FileMeta`] of the file being read
    ///
    /// # Errors
    /// [`ReadError::IoError`](super::errors::ReadError::IoError) if seeking fails.
    pub async fn new(pack: &'r mut R, meta: FileMeta) -> ReadResult<Self> {
        io!(
            pack.seek(SeekFrom::Start(meta.offset)).await,
            ReadStep::CreateDirectFileReader(meta)
        )?;
        Ok(Self {
            pack_file: pack,
            file_meta: meta,
            seek_pos: 0,
        })
    }
}

impl<R> AsyncRead for DirectFileReader<'_, R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_lite::io::Result<usize>> {
        let this = &mut *self;

        let mut pack_file = Pin::new(&mut this.pack_file);

        if this.seek_pos > this.file_meta.size {
            return Poll::Ready(Ok(0));
        }

        let max_read_length = (this.file_meta.size - this.seek_pos) as usize;

        let read_length = min(buf.len(), max_read_length);
        let poll = pack_file.as_mut().poll_read(cx, &mut buf[..read_length]);

        if let Poll::Ready(Ok(read_bytes)) = poll {
            this.seek_pos += read_bytes as u64;
        }

        poll
    }
}

impl<R> AsyncSeek for DirectFileReader<'_, R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: SeekFrom,
    ) -> Poll<futures_lite::io::Result<u64>> {
        let this = self.get_mut();
        let pack_file = Pin::new(&mut this.pack_file);
        match pos {
            SeekFrom::Start(pos) => {
                let pack_seek_pos = this.file_meta.offset + pos;
                this.seek_pos += pos;
                pack_file.poll_seek(cx, SeekFrom::Start(pack_seek_pos))
            }
            SeekFrom::End(pos) => {
                if -pos as i128 > this.file_meta.size as i128 {
                    return Poll::Ready(Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "Tried to seek beyond the start of the file.",
                    )));
                }

                //               equivalent to the end pos
                this.seek_pos = (this.file_meta.size as i128 + pos as i128) as u64;

                let reader_seek_pos = this.file_meta.offset + this.seek_pos;

                pack_file.poll_seek(cx, SeekFrom::Start(reader_seek_pos))
            }
            SeekFrom::Current(pos) => {
                let new_seek_pos = this.seek_pos as i128 + pos as i128;
                if new_seek_pos < 0 {
                    return Poll::Ready(Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "Tried to seek beyond the start of the file.",
                    )));
                }

                this.seek_pos = new_seek_pos as u64;

                let reader_seek_pos = this.file_meta.offset + this.seek_pos;
                pack_file.poll_seek(cx, SeekFrom::Start(reader_seek_pos))
            }
        }
    }
}

/// A [`AsyncRead`] + [`AsyncSeek`] enum used for reading files from asset packs.
///
/// Unlike [`DirectFileReader`], this enum has variants for readers of decompressed files, and normal files.
/// Always use this instead of the [`DirectFileReader`] unless you need access to the compressed
/// data.
pub enum AssetFileReader<'r, R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    /// The [`DirectFileReader`] for an uncompressed file
    Normal(DirectFileReader<'r, R>),
    /// The [`File`] pointing to the decompressed temporary file
    Decompressed(Compat<TempFile>),
}

impl<'r, R: AsyncRead + AsyncSeek + Unpin> AssetFileReader<'r, R> {
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
    pub async fn new(
        file_reader: DirectFileReader<'r, R>,
        file_meta: FileMeta,
    ) -> ReadResult<Self> {
        if file_meta.flags.contains(Flags::COMPRESSED) {
            let mut temp = decompress(file_reader, file_meta).await?;
            io!(
                temp.seek(SeekFrom::Start(0)).await,
                ReadStep::DecompressFile(file_meta)
            )?;
            Ok(AssetFileReader::Decompressed(temp))
        } else {
            Ok(AssetFileReader::Normal(file_reader))
        }
    }
}

impl<'r, R: AsyncRead + AsyncSeek + Unpin> AsyncRead for AssetFileReader<'r, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_lite::io::Result<usize>> {
        let this = self.get_mut();

        match this {
            AssetFileReader::Normal(r) => Pin::new(r).poll_read(cx, buf),
            AssetFileReader::Decompressed(r) => Pin::new(r).poll_read(cx, buf),
        }
    }
}

impl<'r, R: AsyncRead + AsyncSeek + Unpin> AsyncSeek for AssetFileReader<'r, R> {
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: SeekFrom,
    ) -> Poll<futures_lite::io::Result<u64>> {
        let this = self.get_mut();

        match this {
            AssetFileReader::Normal(s) => Pin::new(s).poll_seek(cx, pos),
            AssetFileReader::Decompressed(s) => Pin::new(s).poll_seek(cx, pos),
        }
    }
}
