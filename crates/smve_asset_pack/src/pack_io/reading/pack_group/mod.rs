//! Utilities for reading an asset pack group.

use async_fs::{File, OpenOptions};
use camino::{Utf8Path, Utf8PathBuf};
use futures_lite::io::BufReader;
use futures_lite::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, StreamExt};
use indexmap::IndexMap;
use pathdiff::diff_utf8_paths;
use snafu::{ensure, ResultExt};
use std::collections::HashMap;
use std::io::SeekFrom;
use std::mem;
use tracing::{error, warn};

use async_walkdir::WalkDir;

use crate::pack_io::reading::pack_group::serde::{EnabledPack, EnabledPacks};
use crate::pack_io::reading::{
    AssetFileReader, AssetPackReader, NotADirectoryCtx, ReadResult, ReadStep,
};

use super::utils::io;
use super::{
    ConditionalSendAsyncSeekableBufRead, LoadNotCalledCtx, TomlDeserializeCtx, Utf8PathCtx,
    WalkDirCtx,
};

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
/// # async fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// let mut reader = AssetPackGroupReader::new("custom_packs").await?;
/// # Ok(()) }
/// ```
///
/// Most functions in this struct only registers the changes to happen. They don't actually update
/// the reader immediately. So trying to read files right now through the reader would yield
/// errors.
///
/// To apply the changes done in functions, call `reader.load().await?`.
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
/// # async fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs").await?;
/// reader.add_external_pack("path/to/folder");
/// reader.load().await?;
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
/// use futures_lite::io::Cursor;
/// # macro_rules! include_bytes {
/// #    ($thing:expr) => ("")
/// # }
///
/// # async fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs").await?;
/// // It is recommended to embed the built-in packs in the binary itself, making it harder for
/// // users to modify them and potentially cause problems.
/// reader.register_built_in_pack(
///     "identifier",
///     AssetPackReader::new(Cursor::new(include_bytes!("pack.smap"))).await?.into_dyn_reader()
/// );
/// reader.load().await?;
/// # Ok(()) }
/// ```
///
/// To stop users from modding certain assets, you can also specify an override pack which cannot be
/// disabled and is always at the top of the precedence stack.
///
/// ```no_run
/// # use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
/// # use smve_asset_pack::pack_io::reading::AssetPackReader;
/// # use futures_lite::io::Cursor;
/// # macro_rules! include_bytes {
/// #    ($thing:expr) => ("")
/// # }
/// # async fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs").await?;
/// reader.add_override_pack(AssetPackReader::new(Cursor::new(include_bytes!("pack.smap"))).await?.into_dyn_reader(), "override");
/// reader.load().await?;
/// # Ok(()) }
/// ```
///
/// More calls to [`add_override_pack`](Self::add_override_pack) will put the packs above other
/// pre-existing override packs.
///
/// To change the order of the enabled packs in the precedence stack or to disable and enable
/// packs, you can pass in a slice of paths identifying the packs in order. Built-in packs are
/// referenced by `/__built_in/` followed by the identifier you set earlier when registering the
/// built-in pack.
///
/// ```no_run
/// # use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
/// # async fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs").await?;
/// reader.set_enabled_packs(&[
///     "/__built_in/identifier",
///     "pack1.smap",
///     "external/pack2.smap"
/// ]);
/// reader.load().await?;
/// # Ok(()) }
/// ```
///
/// You can also do the same with override packs:
///
/// ```no_run
/// # use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
/// # async fn blah() -> smve_asset_pack::pack_io::reading::ReadResult<()> {
/// # let mut reader = AssetPackGroupReader::new("custom_packs").await?;
/// reader.rearrange_override_packs(&["override", "override2"]);
/// reader.load().await?;
/// # Ok(()) }
/// ```
pub struct AssetPackGroupReader {
    enabled_packs: EnabledPacks,
    /// This does not include built-in packs
    available_packs: HashMap<Utf8PathBuf, PackDescriptor>,
    external_packs: Vec<Utf8PathBuf>,
    file_name_to_asset_pack: HashMap<Box<str>, PackIndex>,
    packs_changed: bool,
    pack_extension: &'static str,
    root_dir: Utf8PathBuf,
    override_packs:
        IndexMap<Box<str>, AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>>,
}

impl AssetPackGroupReader {
    /// Creates a new [`AssetPackGroupReader`] that contains all .smap files in the specified root_dir.
    ///
    /// # Errors
    /// This will error when encountering IO errors, toml deserialization errors and walkdir errors.
    /// See [`ReadError`] for more information.
    pub async fn new(root_dir: impl AsRef<Utf8Path>) -> ReadResult<Self> {
        let root_dir = root_dir.as_ref();

        ensure!(
            root_dir.is_dir(),
            NotADirectoryCtx {
                path: root_dir.to_path_buf()
            }
        );

        let mut packs_toml = io!(
            OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .truncate(false)
                .open(root_dir.join("packs.toml"))
                .await,
            ReadStep::ReadPacksToml(root_dir.to_path_buf())
        )?;

        // Check if file is empty
        io!(
            packs_toml.seek(SeekFrom::End(0)).await,
            ReadStep::ReadPacksToml(root_dir.to_path_buf())
        )?;

        let position = io!(
            packs_toml.seek(SeekFrom::Current(0)).await,
            ReadStep::ReadPacksToml(root_dir.to_path_buf())
        )?;

        let enabled_packs = if position == 0 {
            // File is empty:
            // It does not currently write anything to the file, as that will happen later
            EnabledPacks::default()
        } else {
            // File isn't empty
            io!(
                packs_toml.seek(SeekFrom::Start(0)).await,
                ReadStep::ReadPacksToml(root_dir.to_path_buf())
            )?;

            let mut opened_packs_str = String::new();
            io!(
                packs_toml.read_to_string(&mut opened_packs_str).await,
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
            override_packs: IndexMap::new(),
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
    pub fn add_external_pack(&mut self, path: impl AsRef<Utf8Path>) {
        self.external_packs.push(path.as_ref().into());
    }

    /// Returns the list of enabled packs, with the first pack having the most precedence.
    pub fn get_enabled_packs(&self) -> &EnabledPacks {
        &self.enabled_packs
    }

    /// Returns all packs that can be found in the root directory and any external packs.
    ///
    /// **NOTE**: This does NOT include built-in packs.
    pub fn get_available_packs(&self) -> &HashMap<Utf8PathBuf, PackDescriptor> {
        &self.available_packs
    }

    /// Returns an asset file reader for a specific file.
    ///
    /// Will return an error if there were any operations after the last call to
    /// [`load`](Self::load).
    pub async fn get_file_reader(
        &mut self,
        file_path: &str,
    ) -> ReadResult<Option<AssetFileReader<'_, Box<dyn ConditionalSendAsyncSeekableBufRead>>>> {
        if self.packs_changed {
            return LoadNotCalledCtx.fail()?;
        }

        let index = self.file_name_to_asset_pack.get(file_path);
        if index.is_none() {
            return Ok(None);
        }

        let pack_reader = match index.unwrap() {
            PackIndex::Enabled(i) => self
                .enabled_packs
                .get_index_mut(*i)
                .unwrap()
                .1
                .pack_reader
                .as_mut()
                .unwrap(),
            PackIndex::OverridePack(i) => self
                .override_packs
                .get_index_mut(*i)
                .map(|(_, reader)| reader)
                .unwrap(),
        };

        pack_reader.get_file_reader(file_path).await
    }

    /// Sets the order of enabled packs, as well as enabling new packs and disabling them.
    ///
    /// Note that this change will not be reflected until [`Self::load`] is called.
    ///
    /// If you have owned paths and you can pass ownership to the function, use
    /// [`set_enabled_packs_owned`](Self::set_enabled_packs_owned) instead.
    ///
    /// # Parameters
    /// - `packs`: An iterator of the Paths of the pack files. The first element of the iterator
    ///   has the most precedence. For built-in asset packs, start the path with "/__built_in" followed
    ///   by the unique identifier you specified when registering it.
    ///
    /// # Information
    /// This will ignore any paths that were not registered in the reader. If you have just added
    /// new packs to the directories, call [`load`](Self::load) first.
    pub fn set_enabled_packs<P>(&mut self, packs: &[P])
    where
        P: AsRef<Utf8Path>,
    {
        self.set_enabled_packs_owned(packs.iter().map(|p| p.as_ref().to_owned()))
    }

    /// A version of [`set_enabled_packs`](Self::set_enabled_packs) that takes in owned paths
    /// instead of references to avoid extra allocations.
    pub fn set_enabled_packs_owned<I>(&mut self, packs: I)
    where
        I: ExactSizeIterator + Iterator<Item = Utf8PathBuf>,
    {
        let mut new_enabled_packs = IndexMap::with_capacity(packs.len());

        for p in packs {
            if self.available_packs.contains_key(&p) {
                if let Some(pack) = self.enabled_packs.swap_remove(&p) {
                    new_enabled_packs.insert(p, pack);
                } else {
                    let pack_descriptor = self.available_packs.get_mut(&p).unwrap();

                    new_enabled_packs.insert(
                        p,
                        EnabledPack {
                            external: pack_descriptor.is_external,
                            pack_reader: None,
                        },
                    );
                    pack_descriptor.enabled = true;
                }
            } else {
                error!("Pack at {p} is not found! Ignoring.");
            }
        }

        // Add any left over built-in packs
        self.enabled_packs
            .retain(|path, _| path.starts_with("/__built_in"));
        mem::swap(&mut self.enabled_packs, &mut new_enabled_packs);
        self.enabled_packs.extend(new_enabled_packs);

        self.packs_changed = true;
    }

    /// Register an asset pack that can be moved up or down the precedence "ladder" but cannot be
    /// disabled. This change will NOT be reflected until [`load`](Self::load) is called.
    ///
    /// The newly added pack will be added to the bottom of the precedence "ladder", unless a pack
    /// under the same name already exists, in which case this will simply update its reader
    /// without moving it in the precedence stack.
    ///
    /// # Parameters
    /// - `identifier`: A path (doesn't need to exist) that uniquely identifies this built-in pack.
    /// - `reader`: The reader for the asset pack. It is recommended that you directly embed this pack in the binary to
    ///   make it more difficult for users to change.
    ///
    /// # Returns
    /// This returns the previous asset pack at the idenfitier if any.
    pub async fn register_built_in_pack(
        &mut self,
        identifier: impl AsRef<Utf8Path>,
        reader: AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>,
    ) -> Option<AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>> {
        let path = Utf8Path::new("/__built_in").join(identifier);

        let pack = self
            .enabled_packs
            .entry(path.clone())
            .or_insert(EnabledPack {
                external: true,
                pack_reader: None,
            });

        let old_reader = pack.pack_reader.replace(reader);

        self.available_packs.insert(
            path,
            PackDescriptor {
                enabled: true,
                is_external: true,
                is_built_in: true,
            },
        );

        self.packs_changed = true;

        old_reader
    }

    /// Remove a built-in asset pack registered through [`register_built_in_pack`](Self::register_built_in_pack).
    /// This change will not be reflected until [`load`](Self::load) is called.
    ///
    /// # Parameters
    /// - `identifier`: The path that was passed into [`register_built_in_pack`](Self::register_built_in_pack) (does not begin with "/__built_in").
    ///
    /// # Returns
    /// The removed asset pack if any.
    pub fn remove_built_in_pack(
        &mut self,
        identifier: impl AsRef<Utf8Path>,
    ) -> Option<AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>> {
        let path = Utf8Path::new("/__built_in").join(identifier);

        self.available_packs.remove(&path);

        self.packs_changed = true;

        self.enabled_packs
            .shift_remove(&path)
            .unwrap()
            .pack_reader
            .take()
    }

    /// Adds an asset pack that stays at the top of the precedence "ladder" and *cannot* be
    /// disabled.
    ///
    /// This is useful for assets which you don't want users to mod.
    ///
    /// This pack will be added above other override packs already present.
    ///
    /// These changes will **NOT** be reflected until [`load`](Self::load) is called.
    ///
    /// # Parameters
    /// - `reader`: An asset pack reader reading the override pack.
    /// - `identifier`: A unique string to identify this override pack.
    ///
    /// # Returns
    /// If there was already an asset pack reader with the same id, the old one will be returned.
    pub fn add_override_pack(
        &mut self,
        reader: AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>,
        identifier: &str,
    ) -> Option<AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>> {
        self.packs_changed = true;
        self.override_packs.insert(Box::from(identifier), reader)
    }

    /// Rearranges the override packs.
    ///
    /// If some existing override packs are not provided in the ids, they will be removed and
    /// returned.
    ///
    /// These changes will **NOT** be reflected until [`load`](Self::load) is called.
    ///
    /// # Parameters
    /// - `ids`: A slice of identifiers for the override packs arranged from highest in the
    ///   precedence stack to lowest in the precedence stack.
    ///
    /// # Returns
    /// [`None`] if no packs were removed, otherwise [`Some`] of all removed readers.
    pub fn rearrange_override_packs<I>(
        &mut self,
        ids: &[I],
    ) -> Option<Vec<AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>>>
    where
        I: AsRef<str>,
    {
        self.packs_changed = true;

        let mut temp_map = IndexMap::new();
        mem::swap(&mut temp_map, &mut self.override_packs);

        for id in ids.iter().rev() {
            let id = id.as_ref();

            if let Some((id, reader)) = temp_map.swap_remove_entry(id) {
                self.override_packs.insert(id, reader);
            } else {
                error!("Override pack {id} is not found! Ignoring.");
            }
        }

        if temp_map.is_empty() {
            None
        } else {
            Some(temp_map.drain(..).map(|(_, reader)| reader).collect())
        }
    }

    /// Removes an override pack and returns the reader.
    ///
    /// This operation is O(n)
    ///
    /// These changes will **NOT** be reflected until [`load`](Self::load) is called.
    ///
    /// # Parameters
    /// - `ids`: The identifiers for the asset pack to be removed.
    ///
    /// # Returns
    /// [`None`] if the pack does not exist, otherwise [`Some`] of the removed pack.
    pub fn remove_override_pack(
        &mut self,
        id: impl AsRef<str>,
    ) -> Option<AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>> {
        self.packs_changed = true;
        self.override_packs.shift_remove(id.as_ref())
    }

    /// Rediscovers all available packs, along with rebuilding the index if the enabled packs has
    /// been changed.
    ///
    /// This function may take a very long time to execute.
    ///
    /// # Errors
    /// This will return an error when encountering IO errors.
    pub async fn load(&mut self) -> ReadResult<()> {
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
        )
        .await?;

        // Discover external packs
        for path in &self.external_packs {
            if !path.exists() {
                warn!("External pack specified at {path} does not exist! Skipping it.",);
                continue;
            }

            if path.is_dir() {
                Self::get_packs_from_dir(
                    &mut self.available_packs,
                    &self.root_dir,
                    path,
                    true,
                    self.pack_extension,
                )
                .await?;
            } else {
                let rel_path = diff_utf8_paths(path, &self.root_dir).unwrap_or(path.clone());

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

        // Used for checking if enabled packs has changed
        let old_enabled_packs_len = self.enabled_packs.len();

        self.enabled_packs
            .retain(|path, _| self.available_packs.contains_key(path));

        if self.packs_changed || old_enabled_packs_len != self.enabled_packs.len() {
            self.file_name_to_asset_pack.clear();

            // Add override files
            for (index, reader) in self.override_packs.values_mut().enumerate().rev() {
                let toc = &reader.get_toc().normal_files;
                for key in toc.keys() {
                    if !self.file_name_to_asset_pack.contains_key(key.as_str()) {
                        self.file_name_to_asset_pack
                            .insert(Box::from(key.as_str()), PackIndex::OverridePack(index));
                    }
                }
            }

            for (index, (path, pack)) in self.enabled_packs.iter_mut().enumerate() {
                if let Some(available_pack) = self.available_packs.get_mut(path) {
                    available_pack.enabled = true;
                }

                if pack.pack_reader.is_none() {
                    let absolute_path = if path.is_absolute() {
                        path
                    } else {
                        &self.root_dir.join(path)
                    };

                    let pack_file = io!(
                        File::open(absolute_path).await,
                        ReadStep::LoadGroupOpenPack(path.clone())
                    )?;
                    let buf_reader = BufReader::new(pack_file);
                    let boxed_buf_reader =
                        Box::new(buf_reader) as Box<dyn ConditionalSendAsyncSeekableBufRead>;

                    pack.pack_reader = Some(AssetPackReader::new(boxed_buf_reader).await?);
                }

                let pack_reader = pack.pack_reader.as_mut().unwrap();
                let toc = pack_reader.get_toc();
                let normal_files = &toc.normal_files;

                for key in normal_files.keys() {
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
                    .open(self.root_dir.join("packs.toml"))
                    .await,
                ReadStep::LoadGroupWritePacksToml(self.root_dir.clone())
            )?;

            io!(
                packs_toml
                    .write_all(
                        "# This file was generated automatically by SMve asset pack.
# Unless you know what you are doing, do NOT modify this file manually as doing so may cause unwanted behavior.\n\n"
                            .as_bytes(),
                    )
                    .await,
                ReadStep::LoadGroupWritePacksToml(self.root_dir.clone())
            )?;

            io!(
                packs_toml
                    .write_all(
                        toml::to_string_pretty(&self.enabled_packs)
                            .unwrap()
                            .as_bytes(),
                    )
                    .await,
                ReadStep::LoadGroupWritePacksToml(self.root_dir.clone())
            )?;

            io!(
                packs_toml.flush().await,
                ReadStep::LoadGroupWritePacksToml(self.root_dir.clone())
            )?;

            self.packs_changed = false;
        }

        Ok(())
    }

    async fn get_packs_from_dir(
        available_packs: &mut HashMap<Utf8PathBuf, PackDescriptor>,
        root_dir: &Utf8Path,
        pack_dir: &Utf8Path,
        is_external: bool,
        extension: &str,
    ) -> ReadResult<()> {
        let mut entries = WalkDir::new(pack_dir);
        while let Some(entry) = entries.next().await {
            let entry = entry.context(WalkDirCtx)?;

            if let Some(path_extension) = entry.path().extension() {
                if path_extension == extension {
                    let entry_path = Utf8PathBuf::try_from(entry.path()).context(Utf8PathCtx)?;
                    let rel_path = diff_utf8_paths(&entry_path, root_dir).unwrap_or(entry_path);

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
    OverridePack(usize),
}
