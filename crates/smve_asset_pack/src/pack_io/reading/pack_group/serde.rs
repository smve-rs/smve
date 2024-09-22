use crate::pack_io::reading::{AssetPackReader, ConditionalSendAsyncSeekableBufRead};
use camino::Utf8PathBuf;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub type EnabledPacks = IndexMap<Utf8PathBuf, EnabledPack>;

#[derive(Serialize, Deserialize, Debug)]
pub struct EnabledPack {
    pub external: bool,
    #[serde(skip)]
    pub pack_reader: Option<AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>>,
}
