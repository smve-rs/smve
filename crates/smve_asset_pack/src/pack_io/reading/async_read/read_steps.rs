use crate::pack_io::reading::async_read::flags::is_unique;
use crate::pack_io::reading::async_read::{DirectFileReader, FileMeta, ReadError, ReadResult};
use async_fs::File;
use blake3::{hash, Hasher};
use futures_lite::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeekExt};
use indexmap::IndexMap;
use lz4::Decoder;
use std::collections::HashMap;
use std::io;
use std::io::SeekFrom;

use super::utils::{read_bytes, read_bytes_and_hash};

pub async fn validate_header<R>(reader: &mut R) -> ReadResult<()>
where
    R: AsyncReadExt + AsyncSeekExt + Unpin,
{
    reader.seek(SeekFrom::Start(0)).await?;

    let header = read_bytes!(reader, 4)?;

    if &header != b"SMAP" {
        return Err(ReadError::InvalidPackFile);
    }

    Ok(())
}

pub async fn validate_version<R>(buf_reader: &mut R) -> ReadResult<u16>
where
    R: AsyncReadExt + AsyncSeekExt + Unpin,
{
    let version = u16::from_be_bytes(read_bytes!(buf_reader, 2)?);

    if version != 1 {
        return Err(ReadError::IncompatibleVersion(version));
    }

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
        let file_name = read_file_name(pack_reader, &mut toc_hasher).await?;
        if file_name.is_none() {
            break;
        }

        let file_meta = read_file_meta(pack_reader, &mut toc_hasher).await?;

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
) -> ReadResult<Option<String>> {
    let mut file_name = vec![];
    pack_reader.read_until(b'\x00', &mut file_name).await?;
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

    let file_name =
        std::str::from_utf8(file_name.as_slice()).map_err(|e| ReadError::Utf8Error {
            source: e,
            path: file_name.clone().into_boxed_slice(),
        })?;
    let file_name = String::from(file_name);

    Ok(Some(file_name))
}

pub async fn read_file_meta<R: AsyncReadExt + Unpin>(
    pack_reader: &mut R,
    toc_hasher: &mut Hasher,
) -> ReadResult<FileMeta> {
    let file_hash = read_bytes_and_hash!(pack_reader, 32, toc_hasher)?;
    let file_flags = read_bytes_and_hash!(pack_reader, 1, toc_hasher)?;
    let file_offset = read_bytes_and_hash!(pack_reader, 8, toc_hasher)?;
    let file_size = read_bytes_and_hash!(pack_reader, 8, toc_hasher)?;

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

pub async fn read_dl<R: AsyncBufReadExt + Unpin>(
    pack_reader: &mut R,
    expected_dl_hash: &[u8],
) -> ReadResult<Vec<String>> {
    let mut dl_hasher = Hasher::new();
    let mut dl = vec![];

    loop {
        let directory_name = read_dl_entry(pack_reader, &mut dl_hasher).await?;
        if directory_name.is_none() {
            break;
        }

        dl.push(directory_name.unwrap());
    }

    let dl_hash = dl_hasher.finalize();
    if &dl_hash != expected_dl_hash {
        return Err(ReadError::DamagedDirectoryList);
    }

    Ok(dl)
}

pub async fn read_dl_entry<R: AsyncBufReadExt + Unpin>(
    pack_reader: &mut R,
    dl_hasher: &mut Hasher,
) -> ReadResult<Option<String>> {
    let mut directory_name = vec![];
    pack_reader.read_until(b'\x00', &mut directory_name).await?;
    dl_hasher.update(&directory_name);
    if directory_name.last() == Some(&0) {
        directory_name.pop();
    } else {
        return Err(ReadError::InvalidPackFile);
    }

    if directory_name.as_slice() == b"\xFF\x10\xFF" {
        // End of DL reached
        return Ok(None);
    }

    let directory_name =
        std::str::from_utf8(directory_name.as_slice()).map_err(|e| ReadError::Utf8Error {
            source: e,
            path: directory_name.clone().into_boxed_slice(),
        })?;
    let directory_name = String::from(directory_name);

    Ok(Some(directory_name))
}

pub async fn validate_files<R: AsyncReadExt + AsyncSeekExt + Unpin>(
    pack_reader: &mut R,
    toc: &mut IndexMap<String, FileMeta>,
    unique_files: &mut HashMap<String, FileMeta>,
) -> ReadResult<()> {
    let file_data_start = pack_reader.seek(SeekFrom::Current(0)).await?;

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

    reader.seek(SeekFrom::Start(0)).await?;

    let mut file_data = vec![];
    reader.read_to_end(&mut file_data).await?;

    let hash = hash(file_data.as_slice());

    if hash != file_meta.hash {
        return Err(ReadError::DamagedFile(file_path.into()));
    }

    Ok(())
}

pub fn get_dir_start_indices(
    directories: &Vec<String>,
    toc: &IndexMap<String, FileMeta>,
) -> HashMap<String, usize> {
    let mut dir_start_indices = HashMap::new();

    for directory in directories {
        for i in 0..toc.len() {
            let (path, _) = toc.get_index(i).expect("Index should be in range.");
            let directory = directory.to_owned();
            if path.starts_with(&(directory.clone() + "/")) {
                dir_start_indices.insert(directory, i);
                break;
            }
        }
    }

    dir_start_indices
}

pub async fn decompress<R>(mut file_reader: R) -> io::Result<File>
where
    R: AsyncRead + Unpin,
{
    let mut buf = vec![];
    file_reader.read_to_end(&mut buf).await?;

    let mut decoder = Decoder::new(buf.as_slice())?;
    let mut output_file = tempfile::tempfile()?;
    io::copy(&mut decoder, &mut output_file)?;

    Ok(output_file.into())
}
