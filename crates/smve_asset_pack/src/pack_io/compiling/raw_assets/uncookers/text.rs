//! Temporary uncooker for text files which obfuscates them

use std::convert::Infallible;

use crate::pack_io::compiling::raw_assets::AssetUncooker;
use crate::util::text_obfuscation::toggle_obfuscation;

/// Asset Uncooker for .txt files
#[derive(Default)]
pub struct TextAssetUncooker;

impl AssetUncooker for TextAssetUncooker {
    type Options = ();
    type Error = Infallible;

    fn uncook(
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
