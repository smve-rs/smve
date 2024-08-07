//! API for compiling asset files

mod compile_steps;
mod errors;
pub mod raw_assets;
mod walk;

pub use errors::*;

use crate::pack_io::compiling::compile_steps::{
    validate_asset_dir, write_assets, write_directory_list, write_hashes, write_header, write_toc,
};
use crate::pack_io::compiling::raw_assets::{AssetUncooker, AssetUncookers};
use std::fs::OpenOptions;
use std::path::Path;

/// Create an instance of this struct to compile an asset pack.
///
/// # Example
/// ```no_run
/// use smve_asset_pack::pack_io::compiling::AssetPackCompiler;
///
/// // Compiles all assets from ./assets into ./assets.smap
/// AssetPackCompiler::new()
///     .compile("./assets", "./assets.smap").unwrap();
/// ```
#[non_exhaustive]
#[derive(Default)]
pub struct AssetPackCompiler {
    asset_uncookers: AssetUncookers,
}

impl AssetPackCompiler {
    /// Create a new [`AssetPackCompiler`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an instance of an asset uncooker for the current compiler.
    pub fn register_asset_uncooker<U>(&mut self, uncooker: U) -> &mut Self
    where
        U: AssetUncooker,
    {
        self.asset_uncookers.register(uncooker);

        self
    }

    /// Initialize an instance of an asset uncooker if it implements [`Default`]
    pub fn init_asset_uncooker<U: AssetUncooker + Default>(&mut self) -> &mut Self {
        self.register_asset_uncooker(U::default())
    }

    /// Compile an asset pack file based on the settings set on the creation of [`AssetPackCompiler`]
    ///
    /// # Parameters
    /// `asset_dir`: Path to a non-empty directory containing the assets
    /// `pack_output`: Path to the output asset pack file
    ///
    /// # Errors
    /// See [`CompileError`] for more information.
    pub fn compile(
        &self,
        asset_dir: impl AsRef<Path>,
        pack_output: impl AsRef<Path>,
    ) -> CompileResult<()> {
        let asset_dir = asset_dir.as_ref();
        let pack_output = pack_output.as_ref();

        validate_asset_dir(asset_dir)?;

        let mut output_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(pack_output)?;

        write_header(&mut output_file)?;

        let (directories, toc_hash, mut file_glob) = write_toc(asset_dir, self, &mut output_file)?;

        let dl_hash = write_directory_list(&directories, &mut output_file)?;

        write_assets(&mut file_glob, &mut output_file)?;

        write_hashes(&mut output_file, toc_hash, dl_hash)?;

        Ok(())
    }
}
