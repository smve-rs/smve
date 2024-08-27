pub mod config;
mod ignore_utils;

use crate::pack_io::compiling::walk::config::glob_utils::glob_matches;
use crate::pack_io::compiling::walk::config::{
    get_dir_config, get_file_config, Configuration, DirectoryConfiguration,
};
use crate::pack_io::compiling::walk::ignore_utils::{get_ignore, get_ignore_with_extra};
use ignore::gitignore::Gitignore;
use merge::Merge;
use std::fs;
use std::fs::{DirEntry, ReadDir};
use std::path::Path;
use tracing::{error, warn};

pub struct Walk<'a> {
    process_stack: Vec<ProcessNode>,
    ignores: Vec<Gitignore>,
    configs: Vec<DirectoryConfiguration<'a>>,
    current_ignores_indices: Vec<usize>,
    current_config_index: usize,
}

impl Walk<'_> {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ignore::Error> {
        Self::new_with_extra_ignores(path, &[])
    }

    pub fn new_with_extra_ignores(
        path: impl AsRef<Path>,
        extra_ignores: &[&str],
    ) -> Result<Self, ignore::Error> {
        let path = path.as_ref();

        let mut process_stack: Vec<ProcessNode> = vec![];
        let mut ignores: Vec<Gitignore> = vec![];
        let mut configs: Vec<DirectoryConfiguration> = vec![];

        process_stack.push(ProcessNode::ReadDir(fs::read_dir(path)?));

        let root_ignore = get_ignore_with_extra(path, extra_ignores).unwrap_or(Gitignore::empty());
        ignores.push(root_ignore);

        let root_config = get_dir_config(path).unwrap_or_default();
        configs.push(root_config);

        Ok(Self {
            process_stack,
            ignores,
            current_ignores_indices: vec![0],
            configs,
            current_config_index: 0,
        })
    }
}

// FIXME: This should also implement FusedIterator
impl<'a> Iterator for Walk<'a> {
    type Item = std::io::Result<(DirEntry, Configuration<'a>)>;

    fn next(&mut self) -> Option<Self::Item> {
        // This will loop until we return something.
        while let Some(top) = self.process_stack.last_mut() {
            match top {
                ProcessNode::ReadDir(read_dir) => {
                    // Unwrap the first dir entry
                    match read_dir.next() {
                        // If there is a DirEntry to be found (with no errors)
                        Some(Ok(entry)) => {
                            return match fs::symlink_metadata(entry.path()) {
                                Ok(metadata) => {
                                    // Skip symlinks
                                    if metadata.is_symlink() {
                                        warn!(
                                            "Skipped symbolic link at {}",
                                            entry.path().display()
                                        );
                                        continue;
                                    }

                                    // Skip configuration and ignore files
                                    let file_name_osstr = entry.file_name();
                                    let file_name = file_name_osstr.to_str();
                                    if file_name.is_none() {
                                        error!("Failed to convert file name to UTF-8! Unexpected behaviour might happen.")
                                    } else if file_name.unwrap().ends_with("__config__.toml")
                                        || file_name.unwrap() == "__ignore__"
                                    {
                                        continue;
                                    }

                                    // Check if this entry should be ignored
                                    // Check from deepest ignore to the top, as deep ignores have precedence
                                    let mut should_ignore = false;
                                    for ignore_index in self.current_ignores_indices.iter().rev() {
                                        let ignore = self
                                            .ignores
                                            .get(*ignore_index)
                                            .expect("Index should exist");
                                        let matched =
                                            ignore.matched(entry.path(), metadata.is_dir());
                                        if !matched.is_none() {
                                            if matched.is_ignore() {
                                                should_ignore = true;
                                            }
                                            break;
                                        }
                                    }
                                    if should_ignore {
                                        continue;
                                    }

                                    // If entry is a directory, push its iterator to the stack
                                    if metadata.is_dir() {
                                        match fs::read_dir(entry.path()) {
                                            Ok(rd) => {
                                                // Try get ignores
                                                let ignore = get_ignore(entry.path());

                                                // Try get config
                                                let mut config = get_dir_config(entry.path());

                                                // Push this before pushing directory, so that after processing this directory we can change back
                                                if ignore.is_some() {
                                                    // We can clone the ignores indices as it usually should only be a few elements long at best.
                                                    self.process_stack.push(
                                                        ProcessNode::IgnoreChange {
                                                            new_indices: self
                                                                .current_ignores_indices
                                                                .clone(),
                                                        },
                                                    );
                                                }

                                                // Same as above
                                                if config.is_some() {
                                                    self.process_stack.push(
                                                        ProcessNode::ConfigChange {
                                                            new_index: self.current_config_index,
                                                        },
                                                    )
                                                }

                                                self.process_stack.push(ProcessNode::ReadDir(rd));

                                                // Push new config
                                                if config.is_some() {
                                                    config.as_mut().unwrap().merge(
                                                        self.configs[self.current_config_index]
                                                            .clone(),
                                                    );

                                                    let new_index = self.configs.len();
                                                    self.configs.push(config.unwrap());
                                                    self.process_stack.push(
                                                        ProcessNode::ConfigChange { new_index },
                                                    )
                                                }

                                                // Push new ignores
                                                if ignore.is_some() {
                                                    let mut new_indices =
                                                        self.current_ignores_indices.clone();
                                                    new_indices.push(self.ignores.len()); // The index where the ignore will be added to
                                                    self.ignores.push(ignore.unwrap());
                                                    self.process_stack.push(
                                                        ProcessNode::IgnoreChange { new_indices },
                                                    );
                                                }
                                            }
                                            Err(e) => return Some(Err(e)),
                                        }
                                    }

                                    // Return the entry
                                    let current_directory_config = self
                                        .configs
                                        .get(self.current_config_index)
                                        .expect("Index should exist.")
                                        .clone();

                                    // This yields an empty configuration struct if a file config couldn't be found.
                                    let mut file_config = if metadata.is_file() {
                                        get_file_config(entry.path())
                                    } else {
                                        None
                                    }
                                    .unwrap_or(Configuration::empty());

                                    let glob_configs = current_directory_config
                                        .glob_configs
                                        .iter()
                                        .cloned() // We need ownership of the configs
                                        // Only collect configs that match with the current path
                                        .filter_map(|(glob, config)| {
                                            if glob_matches(&glob, entry.path()) {
                                                Some(config)
                                            } else {
                                                None
                                            }
                                        });

                                    // Merge every glob config into the one below it (the lower it
                                    // is the MORE precedence it has).
                                    // The top most config will be the current directory config
                                    let mut previous_config =
                                        current_directory_config.dir_config.unwrap();
                                    for mut glob_config in glob_configs.rev() {
                                        glob_config.merge(previous_config);
                                        previous_config = glob_config;
                                    }

                                    // Merge the dir and glob configs into the file config
                                    // File config could be empty, in which case it will just yield
                                    // the dir and glob configs.
                                    file_config.merge(previous_config);

                                    Some(Ok((entry, file_config)))
                                }
                                Err(e) => Some(Err(e)),
                            };
                        }
                        Some(Err(e)) => return Some(Err(e)),
                        // When there are no more items in the current ReadDir, pop it and move on to the next one.
                        None => {
                            self.process_stack.pop();
                        }
                    }
                }
                ProcessNode::IgnoreChange { .. } => {
                    let top = self.process_stack.pop().unwrap();
                    let ProcessNode::IgnoreChange { new_indices } = top else {
                        unreachable!()
                    };
                    self.current_ignores_indices = new_indices;
                }
                ProcessNode::ConfigChange { .. } => {
                    let top = self.process_stack.pop().unwrap();
                    let ProcessNode::ConfigChange { new_index } = top else {
                        unreachable!()
                    };
                    self.current_config_index = new_index;
                }
            }
        }

        // When there are no more items in the stack, return None to end the iterator.
        None
    }
}

pub enum ProcessNode {
    ReadDir(ReadDir),
    IgnoreChange { new_indices: Vec<usize> },
    ConfigChange { new_index: usize },
}
