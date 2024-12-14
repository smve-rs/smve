//! Temporary processor for text files which obfuscates them

use std::convert::Infallible;

use crate::pack_io::compiling::asset_processing::AssetProcessor;
use crate::util::text_obfuscation::toggle_obfuscation;

/// Asset Processor for .txt files
#[derive(Default)]
pub struct TextAssetProcessor;

impl AssetProcessor for TextAssetProcessor {
    type Options = ();
    type Error = Infallible;

    fn process(
        &self,
        buf: &[u8],
        _extension: &str,
        _settings: &Self::Options,
    ) -> Result<Vec<u8>, Self::Error> {
        Ok(toggle_obfuscation(buf))
    }

    fn target_extension(&self) -> &str {
        "smap_text"
    }

    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(["txt"].into_iter())
    }
}
