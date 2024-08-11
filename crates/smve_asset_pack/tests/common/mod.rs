use serde::Deserialize;
use smve_asset_pack::pack_io::compiling::raw_assets::AssetUncooker;

#[derive(Default)]
pub struct EUncooker;

impl AssetUncooker for EUncooker {
    type Options = EUncookerOptions;

    fn uncook(&self, buf: &[u8], _extension: &str, options: &Self::Options) -> Vec<u8> {
        buf.iter().map(|_| options.character).collect()
    }

    fn target_extension(&self) -> &str {
        "e"
    }

    fn source_extensions(&self) -> &[&str] {
        &["txt"]
    }
}

#[derive(Deserialize)]
pub struct EUncookerOptions {
    character: u8,
}

impl Default for EUncookerOptions {
    fn default() -> Self {
        Self { character: b'e' }
    }
}
