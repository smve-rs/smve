//! Contains shared code between compiling and reading

use bitflags::bitflags;

bitflags! {
    /// Code representation of the flags present in asset packs.
    ///
    /// See [File Flags](https://github.com/smve-rs/asset_pack/blob/master/docs/specification/v1.md#file-flags)
    #[derive(Copy, Clone, Debug)]
    pub struct Flags: u8 {
        /// If the asset is stored in its processed and optimised form.
        const PROCESSED = 1 << 0;
        /// If the asset is marked as pack-unique.
        const UNIQUE = 1 << 1;
        /// If the asset is compressed.
        const COMPRESSED = 1 << 2;
    }
}
