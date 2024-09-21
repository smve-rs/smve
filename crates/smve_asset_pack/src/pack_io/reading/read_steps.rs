use crate::pack_io::reading::flags::is_unique;
use crate::pack_io::reading::{
    DamagedFileCtx, DirectFileReader, FileMeta, IncompatibleVersionCtx, InvalidPackFileCtx,
    ReadError, ReadResult, ReadStep,
};
use async_compat::{Compat, CompatExt};
use async_tempfile::TempFile;
use blake3::{hash, Hasher};
use blocking::Unblock;
use futures_lite::{io, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeekExt};
use indexmap::IndexMap;
use lz4::Decoder;
use snafu::{ensure, ResultExt};
use std::collections::HashMap;
use std::io::SeekFrom;

use super::utils::{io, read_bytes, read_bytes_and_hash};
use super::{TempFileCtx, Utf8Ctx};

pub async fn validate_header<R>(reader: &mut R) -> ReadResult<()>
where
    R: AsyncReadExt + AsyncSeekExt + Unpin,
{
    io!(
        reader.seek(SeekFrom::Start(0)).await,
        ReadStep::ValidateHeader
    )?;

    let header = io!(read_bytes!(reader, 4), ReadStep::ValidateHeader)?;

    ensure!(&header == b"SMAP", InvalidPackFileCtx);

    Ok(())
}

pub async fn validate_version<R>(buf_reader: &mut R) -> ReadResult<u16>
where
    R: AsyncReadExt + AsyncSeekExt + Unpin,
{
    let version = u16::from_be_bytes(io!(read_bytes!(buf_reader, 2), ReadStep::ValidateHeader)?);

    ensure!(version == 1, IncompatibleVersionCtx { version });

    Ok(version)
}

pub async fn read_toc<R: AsyncBufReadExt + Unpin>(
    pack_reader: &mut R,
    expected_toc_hash: &[u8],
) -> ReadResult<(IndexMap<String, FileMeta>, HashMap<String, FileMeta>)> {
    let mut toc_hasher = Hasher::new();

    let mut toc = IndexMap::new();
    let mut unique_files = HashMap::new();

    loop {
        let file_name = read_file_name(pack_reader, &mut toc_hasher, toc.len()).await?;
        if file_name.is_none() {
            break;
        }

        let file_meta =
            read_file_meta(pack_reader, &mut toc_hasher, file_name.as_ref().unwrap()).await?;

        if is_unique(file_meta.flags) {
            let file_name = file_name.unwrap();
            file_name
                .strip_prefix("__unique__/")
                .expect("The prefix should exist if it is marked unique.");
            unique_files.insert(file_name, file_meta);
        } else {
            toc.insert(file_name.unwrap(), file_meta);
        }
    }

    let toc_hash = toc_hasher.finalize();
    if &toc_hash != expected_toc_hash {
        return Err(ReadError::DamagedTOC);
    }

    Ok((toc, unique_files))
}

pub async fn read_file_name<R: AsyncBufReadExt + Unpin>(
    pack_reader: &mut R,
    toc_hasher: &mut Hasher,
    index: usize,
) -> ReadResult<Option<String>> {
    let mut file_name = vec![];
    io!(
        pack_reader.read_until(b'\x00', &mut file_name).await,
        ReadStep::ReadTOCEntry(format!("index {index}"))
    )?;
    toc_hasher.update(&file_name);

    if file_name.last() == Some(&0) {
        file_name.pop();
    } else {
        return Err(ReadError::InvalidPackFile);
    }

    if file_name.as_slice() == b"\xFF\x07\xFF" {
        // End of Table of contents reached
        return Ok(None);
    }

    let file_name = std::str::from_utf8(file_name.as_slice()).with_context(|_| Utf8Ctx {
        path: file_name.clone().into_boxed_slice(),
    })?;
    let file_name = String::from(file_name);

    Ok(Some(file_name))
}

pub async fn read_file_meta<R: AsyncReadExt + Unpin>(
    pack_reader: &mut R,
    toc_hasher: &mut Hasher,
    name: &str,
) -> ReadResult<FileMeta> {
    let file_hash = io!(
        read_bytes_and_hash!(pack_reader, 32, toc_hasher),
        ReadStep::ReadTOCEntry(name.to_string())
    )?;
    let file_flags = io!(
        read_bytes_and_hash!(pack_reader, 1, toc_hasher),
        ReadStep::ReadTOCEntry(name.to_string())
    )?;
    let file_offset = io!(
        read_bytes_and_hash!(pack_reader, 8, toc_hasher),
        ReadStep::ReadTOCEntry(name.to_string())
    )?;
    let file_size = io!(
        read_bytes_and_hash!(pack_reader, 8, toc_hasher),
        ReadStep::ReadTOCEntry(name.to_string())
    )?;

    let file_flags = file_flags[0];
    let file_offset = u64::from_be_bytes(file_offset);
    let file_size = u64::from_be_bytes(file_size);

    Ok(FileMeta {
        hash: file_hash,
        flags: file_flags,
        offset: file_offset,
        size: file_size,
    })
}

pub async fn validate_files<R: AsyncReadExt + AsyncSeekExt + Unpin>(
    pack_reader: &mut R,
    toc: &mut IndexMap<String, FileMeta>,
    unique_files: &mut HashMap<String, FileMeta>,
) -> ReadResult<()> {
    let file_data_start = io!(
        pack_reader.seek(SeekFrom::Current(0)).await,
        ReadStep::ValidateFiles
    )?;

    for (path, meta) in toc {
        validate_file(meta, file_data_start, pack_reader, path).await?;
    }

    for (path, meta) in unique_files {
        validate_file(meta, file_data_start, pack_reader, path).await?;
    }

    Ok(())
}

pub async fn validate_file<R: AsyncReadExt + AsyncSeekExt + Unpin>(
    file_meta: &mut FileMeta,
    file_data_start: u64,
    pack_reader: &mut R,
    file_path: &str,
) -> ReadResult<()> {
    file_meta.offset += file_data_start;

    let mut reader = DirectFileReader::new(pack_reader, *file_meta).await?;

    io!(
        reader.seek(SeekFrom::Start(0)).await,
        ReadStep::ValidateFile(file_path.to_string())
    )?;

    let mut file_data = vec![];
    io!(
        reader.read_to_end(&mut file_data).await,
        ReadStep::ValidateFile(file_path.to_string())
    )?;

    let hash = hash(file_data.as_slice());

    ensure!(
        hash == file_meta.hash,
        DamagedFileCtx {
            path: file_path.to_string()
        }
    );

    Ok(())
}

pub async fn decompress<R>(mut file_reader: R, file_meta: FileMeta) -> ReadResult<Compat<TempFile>>
where
    R: AsyncRead + Unpin,
{
    let mut buf = vec![];
    io!(
        file_reader.read_to_end(&mut buf).await,
        ReadStep::DecompressFile(file_meta)
    )?;

    let decoder = io!(
        Decoder::new(std::io::Cursor::new(buf)),
        ReadStep::DecompressFile(file_meta)
    )?;

    let mut decoder = Unblock::new(decoder);

    let mut output_file = TempFile::new()
        .await
        .with_context(|_| TempFileCtx { meta: file_meta })?
        .compat();

    io!(
        io::copy(&mut decoder, &mut output_file).await,
        ReadStep::DecompressFile(file_meta)
    )?;

    Ok(output_file)
}
