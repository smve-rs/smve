use crate::pack_io::reading::flags::is_unique;
use crate::pack_io::reading::{
    DamagedDirectoryListCtx, DamagedFileCtx, DamagedTOCCtx, DirectFileReader, FileMeta,
    IncompatibleVersionCtx, InvalidPackFileCtx, ReadError, ReadResult,
};
use blake3::{hash, Hasher};
use indexmap::IndexMap;
use lz4::Decoder;
use snafu::{ensure, ResultExt};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufRead, Read, Seek, SeekFrom};

use super::utils::{io, read_bytes, read_bytes_and_hash};
use super::{ReadStep, Utf8Ctx};

pub fn validate_header<R>(buf_reader: &mut R) -> ReadResult<()>
where
    R: Read + Seek,
{
    io!(
        buf_reader.seek(SeekFrom::Start(0)),
        ReadStep::ValidateHeader
    )?;

    let header = io!(read_bytes!(buf_reader, 4), ReadStep::ValidateHeader)?;

    ensure!(&header == b"SMAP", InvalidPackFileCtx);

    Ok(())
}

pub fn validate_version(buf_reader: &mut impl Read) -> ReadResult<u16> {
    let version = u16::from_be_bytes(io!(read_bytes!(buf_reader, 2), ReadStep::ValidateHeader)?);

    ensure!(version == 1, IncompatibleVersionCtx { version });

    Ok(version)
}

pub fn read_toc<R: BufRead>(
    pack_reader: &mut R,
    expected_toc_hash: &[u8],
) -> ReadResult<(IndexMap<String, FileMeta>, HashMap<String, FileMeta>)> {
    let mut toc_hasher = Hasher::new();

    let mut toc = IndexMap::new();
    let mut unique_files = HashMap::new();

    loop {
        let file_name = read_file_name(pack_reader, &mut toc_hasher, toc.len())?;
        if file_name.is_none() {
            break;
        }

        let file_meta = read_file_meta(pack_reader, &mut toc_hasher, file_name.as_ref().unwrap())?;

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
    ensure!(&toc_hash == expected_toc_hash, DamagedTOCCtx);

    Ok((toc, unique_files))
}

pub fn read_file_name<R: BufRead>(
    pack_reader: &mut R,
    toc_hasher: &mut Hasher,
    index: usize,
) -> ReadResult<Option<String>> {
    let mut file_name = vec![];
    io!(
        pack_reader.read_until(b'\x00', &mut file_name),
        ReadStep::ReadTOC(format!("index {index}"))
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

pub fn read_file_meta<R: Read>(
    pack_reader: &mut R,
    toc_hasher: &mut Hasher,
    name: &str,
) -> ReadResult<FileMeta> {
    let file_hash = io!(
        read_bytes_and_hash!(pack_reader, 32, toc_hasher),
        ReadStep::ReadTOC(name.to_string())
    )?;
    let file_flags = io!(
        read_bytes_and_hash!(pack_reader, 1, toc_hasher),
        ReadStep::ReadTOC(name.to_string())
    )?;
    let file_offset = io!(
        read_bytes_and_hash!(pack_reader, 8, toc_hasher),
        ReadStep::ReadTOC(name.to_string())
    )?;
    let file_size = io!(
        read_bytes_and_hash!(pack_reader, 8, toc_hasher),
        ReadStep::ReadTOC(name.to_string())
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

pub fn read_dl<R: BufRead>(
    pack_reader: &mut R,
    expected_dl_hash: &[u8],
) -> ReadResult<Vec<String>> {
    let mut dl_hasher = Hasher::new();
    let mut dl = vec![];

    loop {
        let directory_name = read_dl_entry(pack_reader, &mut dl_hasher, dl.len())?;
        if directory_name.is_none() {
            break;
        }

        dl.push(directory_name.unwrap());
    }

    let dl_hash = dl_hasher.finalize();
    ensure!(&dl_hash == expected_dl_hash, DamagedDirectoryListCtx);

    Ok(dl)
}

pub fn read_dl_entry<R: BufRead>(
    pack_reader: &mut R,
    dl_hasher: &mut Hasher,
    index: usize,
) -> ReadResult<Option<String>> {
    let mut directory_name = vec![];
    io!(
        pack_reader.read_until(b'\x00', &mut directory_name),
        ReadStep::ReadDirectoryList(format!("index {index}"))
    )?;

    dl_hasher.update(&directory_name);
    if directory_name.last() == Some(&0) {
        directory_name.pop();
    } else {
        return InvalidPackFileCtx.fail();
    }

    if directory_name.as_slice() == b"\xFF\x10\xFF" {
        // End of DL reached
        return Ok(None);
    }

    let directory_name =
        std::str::from_utf8(directory_name.as_slice()).with_context(|_| Utf8Ctx {
            path: directory_name.clone().into_boxed_slice(),
        })?;
    let directory_name = String::from(directory_name);

    Ok(Some(directory_name))
}

pub fn validate_files<R: Read + Seek>(
    pack_reader: &mut R,
    toc: &mut IndexMap<String, FileMeta>,
    unique_files: &mut HashMap<String, FileMeta>,
) -> ReadResult<()> {
    let file_data_start = io!(pack_reader.stream_position(), ReadStep::ValidateFiles)?;

    for (path, meta) in toc {
        validate_file(meta, file_data_start, pack_reader, path)?;
    }

    for (path, meta) in unique_files {
        validate_file(meta, file_data_start, pack_reader, path)?;
    }

    Ok(())
}

pub fn validate_file<R: Read + Seek>(
    file_meta: &mut FileMeta,
    file_data_start: u64,
    pack_reader: &mut R,
    file_path: &str,
) -> ReadResult<()> {
    file_meta.offset += file_data_start;

    let mut reader = DirectFileReader::new(pack_reader, *file_meta)?;

    io!(
        reader.rewind(),
        ReadStep::ValidateFile(file_path.to_string())
    )?;

    let mut file_data = vec![];
    io!(
        reader.read_to_end(&mut file_data),
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

pub fn decompress<R>(file_reader: R) -> io::Result<File>
where
    R: Read,
{
    let mut decoder = Decoder::new(file_reader)?;
    let mut output_file = tempfile::tempfile()?;
    io::copy(&mut decoder, &mut output_file)?;

    Ok(output_file)
}
