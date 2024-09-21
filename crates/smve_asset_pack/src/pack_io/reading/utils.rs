/// Reading a compile-time constant amount of bytes from a reader.
///
/// # Parameters
/// - `$impl_read`: the reader to read from
/// - `$count`: the amount of bytes to read (has to be a compile-time constant
///
/// # Errors
/// [`std::io::Error`] raised from reading the file.
macro_rules! read_bytes {
    ($impl_read:expr, $count:literal) => {{
        let mut buf = [0u8; $count];
        let result = $impl_read.read_exact(&mut buf).await;
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
macro_rules! read_bytes_and_hash {
    ($impl_read:expr, $count:literal, $hasher:expr) => {{
        let mut buf = [0u8; $count];
        let result = $impl_read.read_exact(&mut buf).await;
        if result.is_err() {
            Err(result.unwrap_err())
        } else {
            $hasher.update(&buf);
            Ok(buf)
        }
    }};
}

/// Wraps something that returns an IO result and calls `with_context` on it with the provided read
/// step.
macro_rules! io {
    ($io:expr, $step:expr) => {{
        use snafu::ResultExt;
        $io.with_context(|_| crate::pack_io::reading::IoCtx { step: $step })
    }};
}

pub(crate) use io;
pub(crate) use read_bytes;
pub(crate) use read_bytes_and_hash;
