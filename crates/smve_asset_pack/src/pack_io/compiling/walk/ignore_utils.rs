use ignore::gitignore::{Gitignore, GitignoreBuilder};
use log::error;
use std::path::Path;

pub fn get_ignore_with_extra(path: impl AsRef<Path>, extra_ignores: &[&str]) -> Option<Gitignore> {
    let ignore_path = path.as_ref().join("__ignore__");

    if ignore_path.exists() && ignore_path.is_file() {
        let mut builder = GitignoreBuilder::new(path);
        if let Some(error) = builder.add(&ignore_path) {
            error!(
                "Error parsing ignore file at {}: {error}\nWill add other globs to ignore.",
                ignore_path.display()
            );
        }

        for extra_ignore in extra_ignores {
            let result = builder.add_line(None, extra_ignore);
            if let Err(error) = result {
                error!("Error parsing extra ignore line {extra_ignore}: {error}\nWill add other lines to ignore.");
            }
        }

        let ignore = builder.build();

        if let Ok(ignore) = ignore {
            Some(ignore)
        } else {
            error!(
                "Error parsing ignore file at {}: {}",
                ignore_path.display(),
                ignore.unwrap_err()
            );
            None
        }
    } else {
        None
    }
}

pub fn get_ignore(path: impl AsRef<Path>) -> Option<Gitignore> {
    get_ignore_with_extra(path, &[])
}
