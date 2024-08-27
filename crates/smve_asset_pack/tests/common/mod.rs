use std::convert::Infallible;

use serde::Deserialize;
use smve_asset_pack::pack_io::compiling::raw_assets::AssetUncooker;

#[derive(Default)]
pub struct EUncooker;

impl AssetUncooker for EUncooker {
    type Options = EUncookerOptions;
    type Error = Infallible;

    fn uncook(
        &self,
        buf: &[u8],
        _extension: &str,
        options: &Self::Options,
    ) -> Result<Vec<u8>, Self::Error> {
        Ok(buf.iter().map(|_| options.character).collect())
    }

    fn target_extension(&self) -> &str {
        "e"
    }

    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(["txt"].into_iter())
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
