//! A moddable asset packing system for SMve.

#[cfg(all(feature = "non_send_readers", feature = "bevy_integration"))]
compile_error!("Bevy integration requires feature non_send_readers to be disabled.");

pub mod pack_io;
pub mod util;
