//! Utilities for reading an asset pack group.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use pathdiff::diff_paths;

use walkdir::WalkDir;

use crate::pack_io::reading::{AssetFileReader, AssetPackReader, ReadResult};
use crate::pack_io::reading::pack_group::serde::EnabledPacks;
use crate::pack_io::reading::ReadError::FileNotFound;

mod serde;

/// TODO: Add documentation when API is complete
pub struct AssetPackGroupReader {
    enabled_packs: EnabledPacks,
    available_packs: HashMap<PathBuf, PackDescriptor>,
    file_name_to_asset_pack: HashMap<Box<str>, usize>,
}

impl AssetPackGroupReader {
    /// Creates a new [`AssetPackGroupReader`] that contains all .smap files in the specified root_dir.
    /// 
    /// # Errors
    /// This will error when encountering IO errors, toml deserialization errors and walkdir errors.
    /// See [`ReadError`] for more information.
    /// 
    /// # Panics
    /// This panics when root_dir is not a directory.
    pub fn new(root_dir: impl AsRef<Path>) -> ReadResult<Self> {
        AssetPackGroupReaderBuilder::new(root_dir)?
            .build()
    }
    
    /// Returns the list of enabled packs, with the first pack having the most precedence.
    pub fn get_enabled_packs(&self) -> &EnabledPacks {
        &self.enabled_packs
    }
    
    /// Returns all packs that can be found in the root directory and any external packs.
    pub fn get_available_packs(&self) -> &HashMap<PathBuf, PackDescriptor> {
        &self.available_packs
    }
    
    /// Returns an asset file reader for a specific file.
    pub fn get_file_reader(&mut self, file_path: &str) -> ReadResult<AssetFileReader> {
        let index = self.file_name_to_asset_pack.get(file_path);
        if index.is_none() {
            return Err(FileNotFound(file_path.into()));
        }
        
        let pack_reader = self.enabled_packs.get_mut(*index.unwrap()).unwrap().pack_reader.as_mut().unwrap();
        
        pack_reader.get_file_reader(file_path)
    }
}

/// A builder for [`AssetPackGroupReader`].
/// 
/// It allows specifying a custom pack extension, and other packs that are not part of the root directory.
pub struct AssetPackGroupReaderBuilder {
    available_packs: HashMap<PathBuf, PackDescriptor>,
    pack_extension: &'static str,
    root_dir: PathBuf,
}

impl AssetPackGroupReaderBuilder {
    /// Creates a new builder with the default extension of `.smap`.
    pub fn new(root_dir: impl AsRef<Path>) -> ReadResult<Self> {
        let root_dir = root_dir.as_ref();

        if !root_dir.is_dir() {
            panic!("{} is not a directory!", root_dir.display());
        }
        
        let mut builder = Self {
            available_packs: HashMap::new(),
            pack_extension: "smap",
            root_dir: root_dir.into()
        };
        
        builder.get_packs_from_dir(root_dir)?;
        
        Ok(builder)
    }

    /// Specify a custom pack extension.
    pub fn with_extension(&mut self, extension: &'static str) -> &mut Self {
        self.pack_extension = extension;

        self
    }

    /// Add an external pack from `pack_path` to the asset pack group. To add a whole folder of
    /// packs, see [`try_add_external_pack_dir`](Self::try_add_external_pack_dir).
    /// 
    /// # Panics
    /// This panics when the pack_path does not point to a file.
    pub fn add_external_pack(&mut self, pack_path: impl AsRef<Path>) -> &mut Self {
        let pack_path = pack_path.as_ref();

        if !pack_path.is_file() {
            panic!("External pack is not a file! If you mean to add a directory, use add_external_pack_dir instead.");
        }
        
        let rel_path = diff_paths(pack_path, &self.root_dir).unwrap_or(pack_path.into());

        self.available_packs.insert(
            rel_path,
            PackDescriptor {
                enabled: false,
                is_external: true,
            },
        );

        self
    }

    /// Tries to add a folder of packs as externals. Will error if recursively reading the directory fails.
    /// 
    /// # Panics
    /// This will panic if pack_dir is not a directory.
    pub fn try_add_external_pack_dir(
        &mut self,
        pack_dir: impl AsRef<Path>,
    ) -> ReadResult<&mut Self> {
        let pack_dir = pack_dir.as_ref();

        if !pack_dir.is_dir() {
            panic!("External pack folder is not a directory! If you mean to add a singular external pack, use add_external_pack instead.");
        }

        self.get_packs_from_dir(pack_dir)?;

        Ok(self)
    }

    // TODO: Revise any owned type as keys in hashmaps, convert to box if needed

    /// Adds a folder of packs as externals, panicking when encountering an error.
    /// 
    /// # Other Panics
    /// This will also panic when pack_dir is not a directory.
    pub fn add_external_pack_dir(&mut self, pack_dir: impl AsRef<Path>) -> &mut Self {
        self.try_add_external_pack_dir(pack_dir).unwrap()
    }

    /// Creates the [`AssetPackGroupReader`], consuming the builder.
    /// 
    /// This will also write to the packs.toml file in the root directory to store information about enabled packs.
    /// 
    /// # Panics
    /// This will panic if root_dir is not a directory.
    /// 
    /// # Errors
    /// This will yield an error if encountering IO errors.
    pub fn build(mut self) -> ReadResult<AssetPackGroupReader> {
        let mut packs_toml = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(false)
            .open(self.root_dir.join("packs.toml"))?;

        // Check if file is empty
        packs_toml.seek(SeekFrom::End(0))?;
        let mut enabled_packs = if packs_toml.stream_position()? == 0 {
            // File is empty:
            // It does not currently write anything to the file, as that will happen later
            EnabledPacks::default()
        } else {
            // File isn't empty
            packs_toml.seek(SeekFrom::Start(0))?;

            let mut opened_packs_str = String::new();
            packs_toml.read_to_string(&mut opened_packs_str)?;

            let enabled_packs: EnabledPacks = toml::from_str(&opened_packs_str)?;

            enabled_packs
        };

        enabled_packs
            .packs
            .retain(|pack| {
                self.available_packs.contains_key(&pack.path)
            });

        let mut file_name_to_asset_pack = HashMap::new();

        for (index, pack) in enabled_packs.packs.iter_mut().enumerate() {
            self.available_packs.get_mut(&pack.path).unwrap().enabled = true;

            let absolute_path = if pack.path.is_absolute() {
                &pack.path
            } else {
                &self.root_dir.join(&pack.path)
            };

            pack.pack_reader = Some(AssetPackReader::new(absolute_path)?);

            let pack_reader = pack.pack_reader.as_mut().unwrap();
            let pack_front = pack_reader.get_pack_front()?;
            let toc = &pack_front.toc;

            for key in toc.keys() {
                if !file_name_to_asset_pack.contains_key(key.as_str()) {
                    file_name_to_asset_pack.insert(Box::from(key.as_str()), index);
                }
            }
        }

        packs_toml.seek(SeekFrom::Start(0))?;
        packs_toml.set_len(0)?;
        packs_toml.write_all(toml::to_string_pretty(&enabled_packs).expect("Format should be correct").as_bytes())?;
        
        Ok(AssetPackGroupReader {
            enabled_packs,
            available_packs: self.available_packs,
            file_name_to_asset_pack,
        })
    }

    fn get_packs_from_dir(&mut self, pack_dir: &Path) -> ReadResult<()> {
        for file in WalkDir::new(pack_dir) {
            let file = file?;

            if file.path().extension().is_none()
                || file.path().extension().unwrap() != self.pack_extension
            {
                continue;
            }
            
            let rel_path = diff_paths(file.path(), &self.root_dir).unwrap_or(file.path().into());
            
            self.available_packs.insert(rel_path, PackDescriptor {
                enabled: false,
                is_external: true,
            });
        }

        Ok(())
    }
}

/// Simple struct that stores information about if the pack is enabled, or if it is external.
#[derive(Debug)]
pub struct PackDescriptor {
    /// If the pack is enabled
    pub enabled: bool,
    /// If the pack is external
    pub is_external: bool,
}
