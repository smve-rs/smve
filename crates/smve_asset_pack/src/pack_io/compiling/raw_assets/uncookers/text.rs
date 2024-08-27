//! Temporary uncooker for text files which obfuscates them

use crate::pack_io::compiling::raw_assets::AssetUncooker;
use crate::util::text_obfuscation::toggle_obfuscation;

/// Asset Uncooker for .txt files
#[derive(Default)]
pub struct TextAssetUncooker;

impl AssetUncooker for TextAssetUncooker {
    type Options = ();

    fn uncook(&self, buf: &[u8], _extension: &str, _settings: &Self::Options) -> Vec<u8> {
        toggle_obfuscation(buf)
    }

    fn target_extension(&self) -> &str {
        "smap_text"
    }

    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(["txt"].into_iter())
    }
}
