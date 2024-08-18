/// A macro that takes something that returns an IO result, and calls `with_context` on them with
/// the provided compile step.
macro_rules! io {
    ($op:expr, $step:expr) => {{
        use snafu::ResultExt;
        $op.with_context(|_| crate::pack_io::compiling::IoCtx { step: $step })
    }};
}

pub(crate) use io;
