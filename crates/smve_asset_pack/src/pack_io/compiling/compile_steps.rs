use crate::pack_io::common::Flags;
use crate::pack_io::compiling::utils::io;
use crate::pack_io::compiling::walk::config::Configuration;
use crate::pack_io::compiling::walk::Walk;
use crate::pack_io::compiling::{
    AssetPackCompiler, CompileResult, CompileStep, EmptyDirectoryCtx, IoCtx, NotADirectoryCtx,
};
use crate::pack_io::utils::WriteExt;
use blake3::{Hash, Hasher};
use lz4::EncoderBuilder;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use snafu::{ensure, ResultExt};
use std::borrow::Cow;
use std::fs::{read, DirEntry, File};
use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::tempfile;
use tracing::error;

use super::{ProcessingCtx, WalkDirCtx};

pub fn validate_asset_dir(asset_dir: &Path) -> CompileResult<()> {
    ensure!(
        asset_dir.is_dir(),
        NotADirectoryCtx {
            path: asset_dir.to_path_buf()
        }
    );

    ensure!(
        io!(
            std::fs::read_dir(asset_dir),
            CompileStep::ValidateAssetDir(asset_dir.to_path_buf())
        )?
        .next()
        .is_some(),
        EmptyDirectoryCtx {
            path: asset_dir.to_path_buf()
        }
    );

    Ok(())
}

pub fn write_header(output_file: &mut File) -> CompileResult<()> {
    // # Header
    // ## Magic
    io!(output_file.write_all(b"SMAP"), CompileStep::WriteHeader)?;
    // ## Version
    io!(
        output_file.write_all(&1_u16.to_be_bytes()),
        CompileStep::WriteHeader
    )?;
    // ## TOC Hash (placeholder)
    io!(output_file.write_all(&[0u8; 32]), CompileStep::WriteHeader)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)] // I don't think there is any way to collapse this further
pub fn process_asset(
    asset: &DirEntry,
    config: Configuration<'_>,
    asset_dir: &Path,
    compiler: &AssetPackCompiler,
    binary_glob: &mut File,
    output_file: &mut File,
    toc_hasher: &mut Hasher,
) -> CompileResult<()> {
    let asset_path = asset.path();
    let relative_path = asset_path
        .strip_prefix(asset_dir)
        // This is according to the documentation of `DirEntry.path()`
        .expect("The path should start with the asset folder.");

    let path_str = relative_path.to_str();
    if path_str.is_none() {
        error!(
            "Path {} could not be converted to UTF-8! Skipping.",
            relative_path.display()
        );
        return Ok(());
    }
    let mut path_str = Cow::from(path_str.unwrap());

    // On windows replace backslash with forward slash to make it compatible with paths
    // generated on unix systems.
    // Don't do this on unix because \ is allowed to be part of the path.
    #[cfg(target_os = "windows")]
    {
        path_str = Cow::from(path_str.replace('\\', "/"));
    }

    if asset.path().is_dir() {
        return Ok(());
    }

    // Data of the current asset file
    let mut file_data = io!(
        read(asset.path()),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;

    let mut flags = Flags::empty();

    // Process the file if processor is enabled
    if config.processor.as_ref().unwrap().enabled.unwrap() {
        let processor = if let Some(processor_path) =
            &config.processor.as_ref().unwrap().processor_path
        {
            let processor = compiler
                .asset_processors
                .get_processor_from_type_name(processor_path);

            if processor.is_none() {
                error!(
                    "Asset processor registered under {processor_path} does not exist!
Available processors are: {:#?}",
                    compiler.asset_processors.get_processor_typenames()
                );
            }

            if let Some(extension) = asset_path.extension() {
                if !processor
                    .unwrap()
                    .source_extensions()
                    .collect::<Vec<_>>()
                    .contains(&extension.to_str().unwrap())
                {
                    error!("Asset processor specified at {processor_path} does not support extension {}!", extension.to_str().unwrap());
                    None
                } else {
                    Some(processor.unwrap())
                }
            } else {
                Some(processor.unwrap())
            }
        } else if let Some(extension) = asset_path.extension() {
            //                                             No UTF-8 error will be emitted
            //                                             because we skipped above if path
            //                                             is not UTF-8
            let extension = extension.to_str().unwrap();
            compiler.asset_processors.get_processor_from_ext(extension)
        } else {
            None
        };

        if let Some(processor) = processor {
            let processor_options = config.processor.unwrap().options.unwrap();

            let deserialized_processor_options =
                processor.try_deserialize_options(processor_options.clone());
            if deserialized_processor_options.is_none() {
                error!("Processor options for {path_str} does not match options expected by the processor for extension {}.
Passed in options: {:#?}", asset_path.extension().unwrap().to_str().unwrap(), processor_options);
            } else {
                file_data = processor
                    .process_dyn(
                        file_data.as_slice(),
                        asset_path.extension().unwrap().to_str().unwrap(),
                        deserialized_processor_options.unwrap().as_ref(),
                    )
                    .context(ProcessingCtx)?;
                flags.insert(Flags::PROCESSED);
                path_str.to_mut().push('.');
                path_str.to_mut().push_str(processor.target_extension());
            }
        }
    }

    // Check if the file is under __unique__
    flags.set(Flags::UNIQUE, path_str.starts_with("__unique__/"));

    // Compress the file if needed
    if config.compression.as_ref().unwrap().enabled.unwrap() {
        file_data = io!(
            compress_asset(&file_data, config.compression.unwrap().level.unwrap()),
            CompileStep::CompressAsset(asset_path.clone())
        )?;
        flags.insert(Flags::COMPRESSED);
    }

    let file_offset = io!(
        binary_glob.stream_position(),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;

    // Hasher for the file data
    let mut file_hasher = Hasher::new();

    // Write and hash the file
    io!(
        binary_glob.write_all_and_hash(&file_data, &mut file_hasher),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;
    let file_hash = file_hasher.finalize();

    // Write easter eggs
    if let Some(messages) = config.super_secret_option {
        if !messages.is_empty() {
            let mut rng = Xoshiro256PlusPlus::seed_from_u64(u64::from_le_bytes(
                file_hash.as_bytes()[0..8].try_into().unwrap(),
            ));

            let message_index = rng.gen_range(0..messages.len());

            let message = messages[message_index];

            io!(
                binary_glob.write_all(message.as_bytes()),
                CompileStep::PreliminaryWrite(asset_path.clone())
            )?;
        }
    }

    // ## File path
    io!(
        output_file.write_all_and_hash(path_str.as_bytes(), toc_hasher),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;
    // ### Null termination
    io!(
        output_file.write_all_and_hash(b"\x00", toc_hasher),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;
    // ## File hash
    io!(
        output_file.write_all_and_hash(file_hash.as_bytes(), toc_hasher),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;
    // ## Flags
    io!(
        output_file.write_all_and_hash(&[flags.bits()], toc_hasher),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;
    // ## File offset
    io!(
        output_file.write_all_and_hash(&file_offset.to_be_bytes(), toc_hasher),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;
    // ## File size
    io!(
        output_file.write_all_and_hash(&(file_data.len() as u64).to_be_bytes(), toc_hasher),
        CompileStep::PreliminaryWrite(asset_path.clone())
    )?;

    Ok(())
}

pub fn compress_asset(mut file_data: &[u8], level: u8) -> io::Result<Vec<u8>> {
    let out = vec![];

    let mut encoder = EncoderBuilder::new().level(level as u32).build(out)?;

    io::copy(&mut file_data, &mut encoder)?;

    let (out, result) = encoder.finish();
    result?;

    Ok(out)
}

pub fn write_toc(
    asset_dir: &Path,
    compiler: &AssetPackCompiler,
    output_file: &mut File,
) -> CompileResult<(Hash, File)> {
    // # Table of Contents
    // Temporary file to append the file data to
    let mut file_glob = tempfile().context(IoCtx {
        step: CompileStep::WriteTOC,
    })?;

    // Hasher for the TOC
    let mut toc_hasher = Hasher::new();

    let assets = Walk::new(asset_dir).context(WalkDirCtx)?;

    // Read every file
    for asset in assets {
        let (asset, config) = asset.context(IoCtx {
            step: CompileStep::WriteTOC,
        })?;

        process_asset(
            &asset,
            config,
            asset_dir,
            compiler,
            &mut file_glob,
            output_file,
            &mut toc_hasher,
        )?;
    }

    // ## End of TOC marker
    output_file
        .write_all_and_hash(b"\xff\x07\xff\x00", &mut toc_hasher)
        .context(IoCtx {
            step: CompileStep::WriteTOC,
        })?;

    Ok((toc_hasher.finalize(), file_glob))
}

pub fn write_assets(file_glob: &mut File, output_file: &mut File) -> CompileResult<()> {
    // ## File glob
    file_glob.rewind().context(IoCtx {
        step: CompileStep::CopyData,
    })?;
    std::io::copy(file_glob, output_file).context(IoCtx {
        step: CompileStep::CopyData,
    })?;

    Ok(())
}

pub fn write_hashes(output_file: &mut File, toc_hash: Hash) -> CompileResult<()> {
    // Write TOC hash
    (|| -> io::Result<()> {
        output_file.seek(SeekFrom::Start(6))?;
        output_file.write_all(toc_hash.as_bytes())?;

        Ok(())
    })()
    .context(IoCtx {
        step: CompileStep::WriteHashes,
    })
}
