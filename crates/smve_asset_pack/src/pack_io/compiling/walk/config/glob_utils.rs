use glob::Pattern;
use std::path::Path;
use tracing::error;

pub fn glob_matches(glob_str: &str, path: impl AsRef<Path>) -> bool {
    let pattern = Pattern::new(glob_str);
    if let Err(error) = pattern {
        error!("{glob_str} is not a valid glob! Pattern Error: {error}");
        return false;
    }

    let path_str = path.as_ref().to_str();
    if path_str.is_none() {
        error!(
            "Path {} contains invalid UTF-8 characters! Skipping glob configs for this path.",
            path.as_ref().display()
        );
        return false;
    }

    pattern.unwrap().matches(path_str.unwrap())
}
