//! Converting assets to their raw forms to store in the asset pack.

use downcast_rs::{Downcast, impl_downcast};
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::error::Error;
use toml::Table;
use tracing::warn;

pub mod processors;

/// Implement this to define how asset files can be converted to their raw forms.
pub trait AssetProcessor {
    /// Settings which the processor takes in. It is deserialized from toml config files in the
    /// assets directory.
    type Options: ProcessorOptions + for<'de> Deserialize<'de> + Default;
    /// Errors that may be encountered during processing.
    type Error: Error + 'static;

    /// Converts the file stored in `buf` into a vector of bytes as the output
    ///
    /// # Parameters
    /// - `buf`: Contains the bytes of the file to be processed
    /// - `extension`: The extension of the file to be processed
    /// - `options`: An instance of the settings struct
    fn process(
        &self,
        buf: &[u8],
        extension: &str,
        options: &Self::Options,
    ) -> Result<Vec<u8>, Self::Error>;

    /// The extension without the leading `.` of the raw file to convert to.
    ///
    /// TODO: Add range of supported extensions
    fn target_extension(&self) -> &str;

    /// A boxed iterator that yields the extensions without the leading `.` of the "cooked" (not-raw) files that can be converted into raw files by this converter.
    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_>;
}

/// Type erased version of [`AssetProcessor`] for storing in a vector.
pub(super) trait AssetProcessorDyn {
    /// Processes the asset stored in `buf` with a dyn options parameter.
    ///
    /// # Parameters
    /// - `buf`: The bytes of the asset file
    /// - `extension`: The extension of the asset file
    /// - `options`: The upcasted options for the asset processor. **Important**: this will panic
    ///   if the passed in processor options is not the one expected by the asset processor. To ensure
    ///   that doesn't happen, pass the value returned by [`Self::try_deserialize_options`].
    fn process_dyn(
        &self,
        buf: &[u8],
        extension: &str,
        options: &dyn ProcessorOptions,
    ) -> Result<Vec<u8>, Box<dyn Error>>;
    /// See [`AssetProcessor::target_extension`].
    fn target_extension(&self) -> &str;
    /// See [`AssetProcessor::source_extensions`].
    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_>;
    /// Deserializes the passed in value into the options type expected by the processor.
    ///
    /// # Parameters
    /// - `table`: A [`toml::Table`] deserialized from the configuration file loaded while compiling
    ///   the asset pack.
    ///
    /// # Returns
    /// Returns the upcasted [`ProcessorOptions`] if deserialization succeeds. Returns [`None`] if deserialization fails.
    fn try_deserialize_options(&self, table: Table) -> Option<Box<dyn ProcessorOptions>>;
}

impl<T> AssetProcessorDyn for T
where
    T: AssetProcessor,
{
    fn process_dyn(
        &self,
        buf: &[u8],
        extension: &str,
        options: &dyn ProcessorOptions,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let options = options
            .downcast_ref::<T::Options>()
            .expect("Settings should match AssetProcessor type");
        T::process(self, buf, extension, options).map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    fn target_extension(&self) -> &str {
        T::target_extension(self)
    }

    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        T::source_extensions(self)
    }

    // Returns none if table cannot be converted to the settings type.
    fn try_deserialize_options(&self, table: Table) -> Option<Box<dyn ProcessorOptions>> {
        let options: Option<T::Options> = if table.is_empty() {
            Some(T::Options::default())
        } else {
            table.try_into().ok()
        };
        options.map(|options| Box::new(options) as Box<dyn ProcessorOptions>)
    }
}

/// This trait is automatically implemented for any struct that can be deserialized into and has
/// a default value.
/// Therefore, any deserializable struct can be used as processor options.
pub trait ProcessorOptions: Downcast {}

impl<T: 'static> ProcessorOptions for T where T: for<'de> Deserialize<'de> + Default {}

impl_downcast!(ProcessorOptions);

/// Stores all the asset processor along with ways to look one up, e.g. extensions and type names.
#[derive(Default)]
pub(super) struct AssetProcessors {
    processors: Vec<Box<dyn AssetProcessorDyn>>,
    extension_to_processors: HashMap<Box<str>, Vec<usize>>,
    type_name_to_processor: HashMap<&'static str, usize>,
}

impl AssetProcessors {
    /// Adds the provided processor into the registry.
    pub fn register<P>(&mut self, processor: P)
    where
        P: AssetProcessor + 'static,
    {
        let processor_index = self.processors.len();

        let type_name = std::any::type_name::<P>();

        self.type_name_to_processor
            .insert(type_name, processor_index);

        for extension in processor.source_extensions() {
            match self.extension_to_processors.entry((*extension).into()) {
                Entry::Occupied(mut entry) => {
                    let processors = entry.get_mut();
                    processors.push(processor_index);
                }
                Entry::Vacant(entry) => {
                    entry.insert(vec![processor_index]);
                }
            }
        }

        self.processors.push(Box::new(processor))
    }

    /// Returns the processor instance associated with its type name, [`None`] if not found.
    pub fn get_processor_from_type_name(&self, type_name: &str) -> Option<&dyn AssetProcessorDyn> {
        let processor_index = self.type_name_to_processor.get(type_name)?;
        Some(&**self.processors.get(*processor_index).unwrap())
    }

    /// Returns one of the processors that can process files of an extension. It will warn
    /// if multiple processors exist for it.
    pub fn get_processor_from_ext(&self, ext: &str) -> Option<&dyn AssetProcessorDyn> {
        let processors = self.extension_to_processors.get(ext)?;

        if processors.len() > 1 {
            warn!(
                "Multiple processors are defined for extension {ext}, please specify one in __config__.toml under processors.processor_path. Will use the first processor registered for this extension."
            );
        }

        Some(&**self.processors.get(processors[0]).unwrap())
    }

    /// Returns all the type names of the registered processors.
    pub fn get_processor_typenames(&self) -> Vec<&str> {
        self.type_name_to_processor.keys().copied().collect()
    }
}
