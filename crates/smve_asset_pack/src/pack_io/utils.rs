use blake3::Hasher;
use std::io::Write;

pub trait WriteExt {
    fn write_all_and_hash(&mut self, buf: &[u8], hasher: &mut Hasher) -> std::io::Result<()>;
}

impl<W> WriteExt for W
where
    W: Write,
{
    fn write_all_and_hash(&mut self, buf: &[u8], hasher: &mut Hasher) -> std::io::Result<()> {
        hasher.update(buf);
        self.write_all(buf)?;
        Ok(())
    }
}

/// Reading a compile-time constant amount of bytes from a reader.
///
/// # Parameters
/// - `$impl_read`: the reader to read from
/// - `$count`: the amount of bytes to read (has to be a compile-time constant
///
/// # Errors
/// [`std::io::Error`] raised from reading the file.
#[macro_export]
macro_rules! read_bytes {
    ($impl_read:expr, $count:literal) => {{
        let mut buf = [0u8; $count];
        let result = $impl_read.read_exact(&mut buf);
        if result.is_err() {
            Err(result.unwrap_err())
        } else {
            Ok(buf)
        }
    }};
}

/// Reading a compile-time constant amount of bytes from a reader, and updates a hasher with the bytes.
///
/// # Parameters
/// - `$impl_read`: the reader to read from
/// - `$count`: the amount of bytes to read (has to be a compile-time constant
/// - `$hasher`: the hasher to update
///
/// # Errors
/// [`std::io::Error`] raised from reading the file.
#[macro_export]
macro_rules! read_bytes_and_hash {
    ($impl_read:expr, $count:literal, $hasher:expr) => {{
        let mut buf = [0u8; $count];
        let result = $impl_read.read_exact(&mut buf);
        if result.is_err() {
            Err(result.unwrap_err())
        } else {
            $hasher.update(&buf);
            Ok(buf)
        }
    }};
}
