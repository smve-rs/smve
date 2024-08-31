use crate::pack_io::reading::async_read::{AssetPackReader, ConditionalSendAsyncSeekableBufRead};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct EnabledPacks {
    #[serde(rename = "pack")]
    pub packs: Vec<EnabledPack>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnabledPack {
    pub path: PathBuf,
    pub external: bool,
    #[serde(skip)]
    pub pack_reader: Option<AssetPackReader<Box<dyn ConditionalSendAsyncSeekableBufRead>>>,
}

impl FromIterator<EnabledPack> for EnabledPacks {
    fn from_iter<T: IntoIterator<Item = EnabledPack>>(iter: T) -> Self {
        Self {
            packs: iter.into_iter().collect(),
        }
    }
}

impl Deref for EnabledPacks {
    type Target = Vec<EnabledPack>;

    fn deref(&self) -> &Self::Target {
        &self.packs
    }
}

impl DerefMut for EnabledPacks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.packs
    }
}

impl From<Vec<EnabledPack>> for EnabledPacks {
    fn from(value: Vec<EnabledPack>) -> Self {
        Self { packs: value }
    }
}
