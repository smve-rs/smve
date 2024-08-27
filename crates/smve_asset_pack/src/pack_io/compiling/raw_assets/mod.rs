//! Converting assets to their raw forms to store in the asset pack.

use downcast_rs::{impl_downcast, Downcast};
use serde::Deserialize;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use toml::Table;
use tracing::warn;

pub mod uncookers;

/// Implement this to define how asset files can be converted to their raw forms.
pub trait AssetUncooker {
    /// Settings which the uncooker takes in. It is deserialized from toml config files in the
    /// assets directory.
    type Options: UncookerOptions + for<'de> Deserialize<'de> + Default;

    /// Converts the file stored in `buf` into a vector of bytes as the output
    ///
    /// # Parameters
    /// - `buf`: Contains the bytes of the file to be uncooked
    /// - `extension`: The extension of the file to be uncooked
    /// - `options`: An instance of the settings struct
    fn uncook(&self, buf: &[u8], extension: &str, options: &Self::Options) -> Vec<u8>;

    /// The extension without the leading `.` of the raw file to convert to.
    ///
    /// TODO: Add range of supported extensions
    fn target_extension(&self) -> &str;

    /// A boxed iterator that yields the extensions without the leading `.` of the "cooked" (not-raw) files that can be converted into raw files by this converter.
    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_>;
}

/// Type erased version of [`AssetUncooker`] for storing in a vector.
pub(super) trait AssetUncookerDyn {
    /// Uncooks the asset stored in `buf` with a dyn options parameter.
    ///
    /// # Parameters
    /// - `buf`: The bytes of the asset file
    /// - `extension`: The extension of the asset file
    /// - `options`: The upcasted options for the asset uncooker. **Important**: this will panic
    ///   if the passed in uncooker options is not the one expected by the asset uncooker. To ensure
    ///   that doesn't happen, pass the value returned by [`Self::try_deserialize_options`].
    fn uncook_dyn(&self, buf: &[u8], extension: &str, options: &dyn UncookerOptions) -> Vec<u8>;
    /// See [`AssetUncooker::target_extension`].
    fn target_extension(&self) -> &str;
    /// See [`AssetUncooker::source_extensions`].
    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_>;
    /// Deserializes the passed in value into the options type expected by the uncooker.
    ///
    /// # Parameters
    /// - `table`: A [`toml::Table`] deserialized from the configuration file loaded while compiling
    ///   the asset pack.
    ///
    /// # Returns
    /// Returns the upcasted [`UncookerOptions`] if deserialization succeeds. Returns [`None`] if deserialization fails.
    fn try_deserialize_options(&self, table: Table) -> Option<Box<dyn UncookerOptions>>;
}

impl<T> AssetUncookerDyn for T
where
    T: AssetUncooker,
{
    fn uncook_dyn(&self, buf: &[u8], extension: &str, options: &dyn UncookerOptions) -> Vec<u8> {
        let options = options
            .downcast_ref::<T::Options>()
            .expect("Settings should match AssetUncooker type");
        T::uncook(self, buf, extension, options)
    }

    fn target_extension(&self) -> &str {
        T::target_extension(self)
    }

    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        T::source_extensions(self)
    }

    // Returns none if table cannot be converted to the settings type.
    fn try_deserialize_options(&self, table: Table) -> Option<Box<dyn UncookerOptions>> {
        let options: Option<T::Options> = if table.is_empty() {
            Some(T::Options::default())
        } else {
            table.try_into().ok()
        };
        options.map(|options| Box::new(options) as Box<dyn UncookerOptions>)
    }
}

/// This trait is automatically implemented for any struct that can be deserialized into and has
/// a default value.
/// Therefore, any deserializable struct can be used as uncooker options.
pub trait UncookerOptions: Downcast {}

impl<T: 'static> UncookerOptions for T where T: for<'de> Deserialize<'de> + Default {}

impl_downcast!(UncookerOptions);

/// Stores all the asset uncookers along with ways to look one up, e.g. extensions and type names.
#[derive(Default)]
pub(super) struct AssetUncookers {
    uncookers: Vec<Box<dyn AssetUncookerDyn>>,
    extension_to_uncookers: HashMap<Box<str>, Vec<usize>>,
    type_name_to_uncooker: HashMap<&'static str, usize>,
}

impl AssetUncookers {
    /// Adds the provided uncooker into the registry.
    pub fn register<U>(&mut self, uncooker: U)
    where
        U: AssetUncooker + 'static,
    {
        let uncooker_index = self.uncookers.len();

        let type_name = std::any::type_name::<U>();

        self.type_name_to_uncooker.insert(type_name, uncooker_index);

        for extension in uncooker.source_extensions() {
            match self.extension_to_uncookers.entry((*extension).into()) {
                Entry::Occupied(mut entry) => {
                    let uncookers = entry.get_mut();
                    uncookers.push(uncooker_index);
                }
                Entry::Vacant(entry) => {
                    entry.insert(vec![uncooker_index]);
                }
            }
        }

        self.uncookers.push(Box::new(uncooker))
    }

    /// Returns the uncooker instance associated with its type name, [`None`] if not found.
    pub fn get_uncooker_from_type_name(&self, type_name: &str) -> Option<&dyn AssetUncookerDyn> {
        let uncooker_index = self.type_name_to_uncooker.get(type_name)?;
        Some(&**self.uncookers.get(*uncooker_index).unwrap())
    }

    /// Returns one of the uncookers that can uncook files of an extension. It will warn
    /// if multiple uncookers exist for it.
    pub fn get_uncooker_from_ext(&self, ext: &str) -> Option<&dyn AssetUncookerDyn> {
        let uncookers = self.extension_to_uncookers.get(ext)?;

        if uncookers.len() > 1 {
            warn!("Multiple uncookers are defined for extension {ext}, please specify one in __config__.toml under uncookers.uncooker_path. Will use the first uncooker registered for this extension.");
        }

        Some(&**self.uncookers.get(uncookers[0]).unwrap())
    }

    /// Returns all the type names of the registered uncookers.
    pub fn get_uncooker_typenames(&self) -> Vec<&str> {
        self.type_name_to_uncooker.keys().copied().collect()
    }
}
