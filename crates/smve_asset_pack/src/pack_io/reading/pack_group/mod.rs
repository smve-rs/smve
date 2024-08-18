//! Utilities for reading an asset pack group.

use log::{error, warn};
use pathdiff::diff_paths;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::pack_io::reading::pack_group::serde::{EnabledPack, EnabledPacks};
use crate::pack_io::reading::{AssetFileReader, AssetPackReader, ReadResult};

use super::utils::io;
use super::{ReadStep, SeekableBufRead, TomlDeserializeCtx, WalkDirCtx};

use snafu::ResultExt;

mod serde;

/// A reader for a directory of asset packs.
///
/// This is used for games which allow users to specify custom asset packs to override built-in
/// ones (modding).
///
/// # How it works
/// To create an [`AssetPackGroupReader`], you need to specify a `root_dir`. This is the directory
/// in which users place their custom pack files, and also where the game stores information about
/// what packs are enabled and which have higher precedence than others.
///
/// If a file appears in two or more packs, the version from the higher precedence pack will
/// override the lower precedence packs.
///
/// ```no_run
/// use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
///
/// # fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// let mut reader = AssetPackGroupReader::new("custom_packs")?;
/// # Ok(()) }
/// ```
///
/// Most functions in this struct only registers the changes to happen. They don't actually update
/// the reader immediately. So trying to read files right now through the reader would yield
/// errors.
///
/// To apply the changes done in functions, call `reader.load()?`.
///
/// This is because `reader.load()` is very expensive most of the time, having to recursively
/// discover files in directories and building the index from file paths to their corresponding
/// packs. It is way more efficient to queue up multiple changes, and then apply them at the same
/// time.
///
/// You can also specify other locations where asset packs can be found. This might be useful if
/// you have mods that contain their own asset packs.
///
/// ```no_run
/// use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
///
/// # fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs")?;
/// reader.add_external_pack("path/to/folder");
/// reader.load()?;
/// # Ok(()) }
/// ```
///
/// To avoid users accidentally (or purposefully) disabling built-in asset packs causing certain
/// assets to be missing, you can register built-in packs. They can be moved up and down the
/// precedence stack, but cannot be disabled.
///
/// ```no_run
/// # use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
/// use smve_asset_pack::pack_io::reading::AssetPackReader;
/// use std::io::Cursor;
/// # macro_rules! include_bytes {
/// #    ($thing:expr) => ("")
/// # }
///
/// # fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs")?;
/// // It is recommended to embed the built-in packs in the binary itself, making it harder for
/// // users to modify them and potentially cause problems.
/// reader.register_built_in_pack(
///     "identifier",
///     AssetPackReader::new(Cursor::new(include_bytes!("pack.smap")))?.box_reader()
/// );
/// reader.load()?;
/// # Ok(()) }
/// ```
///
/// To stop users from modding certain assets, you can also specify an override pack which cannot be
/// disabled and is always at the top of the precedence stack.
///
/// ```no_run
/// # use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
/// # use smve_asset_pack::pack_io::reading::AssetPackReader;
/// # use std::io::Cursor;
/// # macro_rules! include_bytes {
/// #    ($thing:expr) => ("")
/// # }
/// # fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs")?;
/// reader.set_override_pack(AssetPackReader::new(Cursor::new(include_bytes!("pack.smap")))?.box_reader());
/// reader.load()?;
/// # Ok(()) }
/// ```
///
/// To change the order of the enabled packs in the precedence stack or to disable and enable
/// packs, you can pass in a slice of paths identifying the packs in order. Built-in packs are
/// referenced by `/__built_in/` followed by the identifier you set earlier when registering the
/// built-in pack.
///
/// ```no_run
/// # use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
/// # fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs")?;
/// reader.set_enabled_packs(&["/__built_in/identifier", "pack1.smap", "external/pack2.smap"]);
/// reader.load()?;
/// # Ok(()) }
/// ```
pub struct AssetPackGroupReader {
    enabled_packs: EnabledPacks,
    /// This does not include built-in packs
    available_packs: HashMap<PathBuf, PackDescriptor>,
    external_packs: Vec<PathBuf>,
    file_name_to_asset_pack: HashMap<Box<str>, PackIndex>,
    packs_changed: bool,
    pack_extension: &'static str,
    root_dir: PathBuf,
    override_pack: Option<AssetPackReader<Box<dyn SeekableBufRead>>>,
}

impl AssetPackGroupReader {
    /// Creates a new [`AssetPackGroupReader`] that contains all .smap files in the specified root_dir.
    ///
    /// # Errors
    /// This will error when encountering IO errors, toml deserialization errors and walkdir errors.
    /// See [`ReadError`](super::ReadError) for more information.
    ///
    /// # Panics
    /// This panics when root_dir is not a directory.
    pub fn new(root_dir: impl AsRef<Path>) -> ReadResult<Self> {
        let root_dir = root_dir.as_ref();

        // TODO: Remove panic
        if !root_dir.is_dir() {
            panic!("{} is not a directory!", root_dir.display());
        }

        let mut packs_toml = io!(
            OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .truncate(false)
                .open(root_dir.join("packs.toml")),
            ReadStep::ReadPacksToml(root_dir.to_path_buf())
        )?;

        // Check if file is empty
        io!(
            packs_toml.seek(SeekFrom::End(0)),
            ReadStep::ReadPacksToml(root_dir.to_path_buf())
        )?;

        let position = io!(
            packs_toml.stream_position(),
            ReadStep::ReadPacksToml(root_dir.to_path_buf())
        )?;

        let enabled_packs = if position == 0 {
            // File is empty:
            // It does not currently write anything to the file, as that will happen later
            EnabledPacks::default()
        } else {
            // File isn't empty
            io!(
                packs_toml.seek(SeekFrom::Start(0)),
                ReadStep::ReadPacksToml(root_dir.to_path_buf())
            )?;

            let mut opened_packs_str = String::new();
            io!(
                packs_toml.read_to_string(&mut opened_packs_str),
                ReadStep::ReadPacksToml(root_dir.to_path_buf())
            )?;

            let enabled_packs: EnabledPacks =
                toml::from_str(&opened_packs_str).with_context(|_| TomlDeserializeCtx {
                    path: root_dir.to_path_buf(),
                })?;

            enabled_packs
        };

        Ok(Self {
            enabled_packs,
            external_packs: vec![],
            available_packs: HashMap::new(),
            file_name_to_asset_pack: HashMap::new(),
            packs_changed: true,
            pack_extension: "smap",
            root_dir: root_dir.into(),
            override_pack: None,
        })
    }

    /// Sets the extension of pack files to look for.
    ///
    /// Note that this change will not be reflected until [`Self::load`] is called.
    pub fn set_pack_extension(&mut self, ext: &'static str) {
        self.pack_extension = ext;
    }

    /// Adds an external pack source to the reader.
    ///
    /// Note that this function simply registers the path as an external pack source. It does not
    /// check the validity of the path. The path will only be processed after
    /// [`load`](AssetPackGroupReader::load) is called on the reader.
    ///
    /// # Parameters
    /// - `path`: **This needs to be relative to the working directory of the application.**
    ///   Can be either a directory or a file. If it is a directory, when
    ///   [`load`](AssetPackGroupReader::load) is called, any file in the directory with the
    ///   correct extension will be marked as an available pack. If it is a file, it will be read
    ///   as a pack file regardless of the extension.
    pub fn add_external_pack(&mut self, path: impl AsRef<Path>) {
        self.external_packs.push(path.as_ref().into());
    }

    /// Returns the list of enabled packs, with the first pack having the most precedence.
    pub fn get_enabled_packs(&self) -> &EnabledPacks {
        &self.enabled_packs
    }

    /// Returns all packs that can be found in the root directory and any external packs.
    ///
    /// **NOTE**: This does NOT include built-in packs.
    pub fn get_available_packs(&self) -> &HashMap<PathBuf, PackDescriptor> {
        &self.available_packs
    }

    /// Returns an asset file reader for a specific file.
    pub fn get_file_reader(
        &mut self,
        file_path: &str,
    ) -> ReadResult<Option<AssetFileReader<Box<dyn SeekableBufRead>>>> {
        let index = self.file_name_to_asset_pack.get(file_path);
        if index.is_none() {
            return Ok(None);
        }

        let pack_reader = match index.unwrap() {
            PackIndex::Enabled(i) => self
                .enabled_packs
                .get_mut(*i)
                .unwrap()
                .pack_reader
                .as_mut()
                .unwrap(),
            PackIndex::OverridePack => self.override_pack.as_mut().unwrap(),
        };

        pack_reader.get_file_reader(file_path)
    }

    /// Sets the order of enabled packs, as well as enabling new packs and disabling them.
    ///
    /// Note that this change will not be reflected until [`Self::load`] is called.
    ///
    /// # Parameters
    /// - `packs`: An ordered slice of the Paths of the pack files. The first element of the slice
    ///   has the most precedence. For built-in asset packs, start the path with "/__built_in" followed
    ///   by the unique identifier you specified when registering it.
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

        // Add any left over built-in packs
        hashmap.retain(|path, _| path.starts_with("/__built_in"));
        new_packs.extend(hashmap.into_values());

        self.enabled_packs = new_packs.into();

        error!("{:?}", self.enabled_packs);

        self.packs_changed = true;
    }

    /// Register an asset pack that can be moved up or down the precedence "ladder" but cannot be
    /// disabled. This change will NOT be reflected until [`load`](Self::load) is called.
    ///
    /// # Parameters
    /// - `identifier`: A path (doesn't need to exist) that uniquely identifies this built-in pack.
    /// - `reader`: The reader for the asset pack. It is recommended that you directly embed this pack in the binary to
    ///   make it more difficult for users to change.
    ///
    /// # Returns
    /// This returns the existing asset pack at the identifier if it already exists.
    pub fn register_built_in_pack(
        &mut self,
        identifier: impl AsRef<Path>,
        reader: AssetPackReader<Box<dyn SeekableBufRead>>,
    ) -> Option<AssetPackReader<Box<dyn SeekableBufRead>>> {
        let path = Path::new("/__built_in").join(identifier);

        let old_pack = if let Some(pack) = self.enabled_packs.iter_mut().find(|p| p.path == path) {
            let old_pack = pack.pack_reader.take();
            pack.pack_reader = Some(reader);
            old_pack
        } else {
            self.enabled_packs.push(EnabledPack {
                path: path.clone(),
                external: true,
                pack_reader: Some(reader),
            });
            None
        };
        self.available_packs.insert(
            path,
            PackDescriptor {
                enabled: true,
                is_external: true,
                is_built_in: true,
            },
        );

        self.packs_changed = true;

        old_pack
    }

    /// Remove a built-in asset pack registered through [`register_built_in_pack`](Self::register_built_in_pack).
    /// This change will not be reflected until [`load`](Self::load) is called.
    ///
    /// # Parameters
    /// - `identifier`: The path that was passed into [`register_built_in_pack`](Self::register_built_in_pack). (Does not begin with "/__built_in")
    ///
    /// # Returns
    /// Returns the removed asset pack if any.
    pub fn remove_built_in_pack(
        &mut self,
        identifier: impl AsRef<Path>,
    ) -> Option<AssetPackReader<Box<dyn SeekableBufRead>>> {
        let path = Path::new("/__built_in").join(identifier);

        let old_pack_index = self.enabled_packs.iter().position(|p| p.path == path)?;

        self.available_packs.remove(&path);

        self.packs_changed = true;

        self.enabled_packs.remove(old_pack_index).pack_reader.take()
    }

    /// Register an asset pack that stays at the top of the precedence "ladder" and *cannot* be
    /// disabled.
    ///
    /// This is useful for assets which you don't want users to mod.
    ///
    /// These changes will **NOT** be reflected until [`load`](Self::load) is called.
    ///
    /// # Parameters
    /// - `reader`: An asset pack reader reading the override pack.
    ///
    /// # Returns
    /// The old override pack if it already exists, [`None`] otherwise.
    pub fn set_override_pack(
        &mut self,
        reader: AssetPackReader<Box<dyn SeekableBufRead>>,
    ) -> Option<AssetPackReader<Box<dyn SeekableBufRead>>> {
        let existing = self.override_pack.take();

        self.override_pack = Some(reader);

        self.packs_changed = true;

        existing
    }

    /// Removes the registered override pack.
    ///
    /// Note that these changes will not be reflected until [`Self::load`] is called.
    pub fn remove_override_pack(&mut self) -> Option<AssetPackReader<Box<dyn SeekableBufRead>>> {
        let old = self.override_pack.take();

        if old.is_some() {
            self.packs_changed = true;
        }

        old
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
        self.available_packs
            .retain(|path, _| path.starts_with("/__built_in"));

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
                        is_built_in: false,
                    },
                );
            }
        }

        self.enabled_packs
            .retain(|pack| self.available_packs.contains_key(&pack.path));

        if self.packs_changed {
            self.file_name_to_asset_pack.clear();

            // Add override files
            if let Some(ref mut override_pack) = self.override_pack {
                let toc = &override_pack.get_pack_front()?.toc;
                for key in toc.keys() {
                    self.file_name_to_asset_pack
                        .insert(Box::from(key.as_str()), PackIndex::OverridePack);
                }
            }

            for (index, pack) in self.enabled_packs.packs.iter_mut().enumerate() {
                if let Some(available_pack) = self.available_packs.get_mut(&pack.path) {
                    available_pack.enabled = true;
                }

                if pack.pack_reader.is_none() {
                    let absolute_path = if pack.path.is_absolute() {
                        &pack.path
                    } else {
                        &self.root_dir.join(&pack.path)
                    };

                    let pack_file = io!(
                        File::open(absolute_path),
                        ReadStep::LoadGroupOpenPack(pack.path.clone())
                    )?;
                    let buf_reader = BufReader::new(pack_file);
                    let boxed_buf_reader = Box::new(buf_reader) as Box<dyn SeekableBufRead>;

                    pack.pack_reader = Some(AssetPackReader::new(boxed_buf_reader)?);
                }

                let pack_reader = pack.pack_reader.as_mut().unwrap();
                let pack_front = pack_reader.get_pack_front()?;
                let toc = &pack_front.toc;

                for key in toc.keys() {
                    if !self.file_name_to_asset_pack.contains_key(key.as_str()) {
                        self.file_name_to_asset_pack
                            .insert(Box::from(key.as_str()), PackIndex::Enabled(index));
                    }
                }
            }

            let mut packs_toml = io!(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(self.root_dir.join("packs.toml")),
                ReadStep::LoadGroupWritePacksToml(self.root_dir.clone())
            )?;

            io!(
                packs_toml.write_all(
                    "# This file was generated automatically by SMve asset pack.
# Do NOT modify this file manually as doing so may cause unwanted behaviour.\n\n"
                        .as_bytes(),
                ),
                ReadStep::LoadGroupWritePacksToml(self.root_dir.clone())
            )?;

            io!(
                packs_toml.write_all(
                    toml::to_string_pretty(&self.enabled_packs)
                        .unwrap()
                        .as_bytes(),
                ),
                ReadStep::LoadGroupWritePacksToml(self.root_dir.clone())
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
            let entry = entry.context(WalkDirCtx)?;

            if let Some(path_extension) = entry.path().extension() {
                if path_extension == extension {
                    let rel_path =
                        diff_paths(entry.path(), root_dir).unwrap_or(entry.path().into());

                    available_packs.insert(
                        rel_path,
                        PackDescriptor {
                            enabled: false,
                            is_external,
                            is_built_in: false,
                        },
                    );
                }
            }
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
    /// If the pack is built in
    pub is_built_in: bool,
}

enum PackIndex {
    Enabled(usize),
    OverridePack,
}
