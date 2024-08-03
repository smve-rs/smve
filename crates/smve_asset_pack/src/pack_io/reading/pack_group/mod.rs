//! Utilities for reading an asset pack group.

use log::{error, warn};
use pathdiff::diff_paths;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::pack_io::reading::pack_group::serde::{EnabledPack, EnabledPacks};
use crate::pack_io::reading::ReadError::FileNotFound;
use crate::pack_io::reading::{AssetFileReader, AssetPackReader, ReadResult};

mod serde;

/// TODO: Add documentation when API is complete
pub struct AssetPackGroupReader {
    enabled_packs: EnabledPacks,
    available_packs: HashMap<PathBuf, PackDescriptor>,
    external_packs: Vec<PathBuf>,
    file_name_to_asset_pack: HashMap<Box<str>, usize>,
    packs_changed: bool,
    pack_extension: &'static str,
    root_dir: PathBuf,
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
        AssetPackGroupReaderBuilder::new(root_dir)?.build()
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

        let pack_reader = self
            .enabled_packs
            .get_mut(*index.unwrap())
            .unwrap()
            .pack_reader
            .as_mut()
            .unwrap();

        pack_reader.get_file_reader(file_path)
    }

    /// Sets the order of enabled packs, as well as enabling new packs and disabling them.
    ///
    /// # Parameters
    /// - `packs`: An ordered slice of the Paths of the pack files.
    ///
    /// # Information
    /// This will ignore any paths that were not registered in the reader. If you have just added
    /// new packs to the directories, call [`load`](Self::load) first.
    pub fn set_enabled_packs<P>(&mut self, packs: &[P])
    where
        P: AsRef<Path>,
    {
        let mut hashmap: HashMap<_, _> = self
            .enabled_packs
            .drain(..)
            .map(|p| (p.path.clone(), p))
            .collect();

        let mut new_packs = Vec::with_capacity(packs.len());

        for pack in packs {
            let pack = pack.as_ref();

            if let Some(p) = hashmap.remove(pack) {
                new_packs.push(p);
            } else {
                let pack_descriptor = self.available_packs.get_mut(pack);

                if let Some(pack_descriptor) = pack_descriptor {
                    new_packs.push(EnabledPack {
                        path: pack.into(),
                        external: pack_descriptor.is_external,
                        pack_reader: None,
                    });
                    pack_descriptor.enabled = true;
                } else {
                    error!("Pack at {} is not found! Ignoring.", pack.display());
                }
            }
        }

        self.enabled_packs = new_packs.into();

        self.packs_changed = true;
    }

    /// Rediscovers all available packs, along with rebuilding the index if the enabled packs has
    /// been changed.
    ///
    /// This function may take a very long time to execute.
    ///
    /// # Errors
    /// This will return an error when encountering IO errors.
    pub fn load(&mut self) -> ReadResult<()> {
        // Rediscover packs
        self.available_packs.clear();

        // Discover root directory packs
        Self::get_packs_from_dir(
            &mut self.available_packs,
            &self.root_dir,
            &self.root_dir,
            false,
            self.pack_extension,
        )?;

        // Discover external packs
        for path in &self.external_packs {
            if !path.exists() {
                warn!(
                    "External pack specified at {} does not exist! Skipping it.",
                    path.display()
                );
                continue;
            }

            if path.is_dir() {
                Self::get_packs_from_dir(
                    &mut self.available_packs,
                    &self.root_dir,
                    path,
                    true,
                    self.pack_extension,
                )?;
            } else {
                let rel_path = diff_paths(path, &self.root_dir).unwrap_or(path.clone());

                self.available_packs.insert(
                    rel_path,
                    PackDescriptor {
                        enabled: false,
                        is_external: true,
                    },
                );
            }
        }

        self.enabled_packs
            .retain(|pack| self.available_packs.contains_key(&pack.path));

        if self.packs_changed {
            self.file_name_to_asset_pack.clear();

            for (index, pack) in self.enabled_packs.packs.iter_mut().enumerate() {
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
                    println!("{key}");
                    if !self.file_name_to_asset_pack.contains_key(key.as_str()) {
                        self.file_name_to_asset_pack
                            .insert(Box::from(key.as_str()), index);
                    }
                }
            }

            let mut packs_toml = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(self.root_dir.join("packs.toml"))?;

            packs_toml.write_all(
                toml::to_string_pretty(&self.enabled_packs)
                    .unwrap()
                    .as_bytes(),
            )?;

            self.packs_changed = false;
        }

        Ok(())
    }

    fn get_packs_from_dir(
        available_packs: &mut HashMap<PathBuf, PackDescriptor>,
        root_dir: &Path,
        pack_dir: &Path,
        is_external: bool,
        extension: &str,
    ) -> ReadResult<()> {
        for entry in WalkDir::new(pack_dir) {
            let entry = entry?;

            if let Some(path_extension) = entry.path().extension() {
                if path_extension == extension {
                    let rel_path =
                        diff_paths(entry.path(), root_dir).unwrap_or(entry.path().into());

                    available_packs.insert(
                        rel_path,
                        PackDescriptor {
                            enabled: false,
                            is_external,
                        },
                    );
                }
            }
        }

        Ok(())
    }
}

/// A builder for [`AssetPackGroupReader`].
///
/// It allows specifying a custom pack extension, and other packs that are not part of the root directory.
pub struct AssetPackGroupReaderBuilder {
    external_packs: Vec<PathBuf>,
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

        Ok(Self {
            external_packs: vec![],
            pack_extension: "smap",
            root_dir: root_dir.into(),
        })
    }

    /// Specify a custom pack extension.
    pub fn with_extension(&mut self, extension: &'static str) -> &mut Self {
        self.pack_extension = extension;

        self
    }

    /// Adds an external pack source to the reader.
    ///
    /// Note that this function simply registers the path as an external pack source. It does not
    /// check the validity of the path. The path will only be processed after
    /// [`load`](AssetPackGroupReader::load) is called on the built reader.
    ///
    /// # Parameters
    /// - `path`: **This needs to be relative to the working directory of the application.**
    ///   Can be either a directory or a file. If it is a directory, when
    ///   [`load`](AssetPackGroupReader::load) is called, any file in the directory with the
    ///   correct extension will be marked as an available pack. If it is a file, it will be read
    ///   as a pack file regardless of the extension.
    pub fn add_external_pack(&mut self, path: impl AsRef<Path>) -> &mut Self {
        let path = path.as_ref();

        self.external_packs.push(path.into());

        self
    }

    // TODO: Revise any owned type as keys in hashmaps, convert to box if needed

    /// Creates the [`AssetPackGroupReader`], consuming the builder.
    ///
    /// This will also write to the packs.toml file in the root directory to store information about enabled packs.
    ///
    /// # Panics
    /// This will panic if root_dir is not a directory.
    ///
    /// # Errors
    /// This will yield an error if encountering IO errors.
    pub fn build(self) -> ReadResult<AssetPackGroupReader> {
        let mut packs_toml = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(false)
            .open(self.root_dir.join("packs.toml"))?;

        // Check if file is empty
        packs_toml.seek(SeekFrom::End(0))?;
        let enabled_packs = if packs_toml.stream_position()? == 0 {
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

        Ok(AssetPackGroupReader {
            enabled_packs,
            external_packs: vec![],
            available_packs: HashMap::new(),
            file_name_to_asset_pack: HashMap::new(),
            packs_changed: true,
            pack_extension: self.pack_extension,
            root_dir: self.root_dir,
        })
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
