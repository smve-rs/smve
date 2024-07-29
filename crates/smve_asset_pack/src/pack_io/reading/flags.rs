//! Functions to check for bits of the file flags stored in the TOC.

/// Returns true if the asset is stored in its raw form in the asset pack (AKA it has been uncooked).
///
/// # Parameters
/// - `flags`: The flags contained in [`FileMeta.flags`](super::FileMeta).
pub fn is_raw(flags: u8) -> bool {
    flags & 0x01 != 0
}

/// Returns true if the asset is marked as pack-unique.
///
/// # Parameters
/// - `flags`: The flags contained in [`FileMeta.flags`](super::FileMeta).
pub fn is_unique(flags: u8) -> bool {
    flags & 0x02 != 0
}

/// Returns true if the asset is compressed.
///
/// # Parameters
/// - `flags` The flags contained in [`FileMeta.flags`](super::FileMeta).
pub fn is_compressed(flags: u8) -> bool {
    flags & 0x04 != 0
}
