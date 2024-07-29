use crate::pack_io::compiling::walk::config::DirectoryConfiguration;
use log::error;
use std::path::Path;
use toml::{Table, Value};

pub fn merge_table(higher: &mut Table, lower: Table) {
    for (key, value) in lower {
        if !higher.contains_key(&key) {
            higher.insert(key, value);
            continue;
        }

        let a_value = higher.get_mut(&key).unwrap();

        if let (Value::Table(a_table), Value::Table(b_table)) = (a_value, value) {
            merge_table(a_table, b_table);
        }
    }
}

pub fn merge_option_table(higher: &mut Option<Table>, lower: Option<Table>) {
    if higher.is_none() {
        *higher = lower;
    } else if let (Some(a_inner), Some(b_inner)) = (higher, lower) {
        merge_table(a_inner, b_inner);
    }
}

impl DirectoryConfiguration<'_> {
    pub fn merge_directory_config(&mut self, lower: Self, directory_path: impl AsRef<Path>) {
        merge::option::recurse(&mut self.dir_config, lower.dir_config);
        let directory_path = directory_path.as_ref();

        if !directory_path.is_dir() {
            error!(
                "Failed to merge glob configurations!
You probably passed in the wrong path to merge_directory_config.
The passed in path is {}
Ignoring new glob configs.",
                directory_path.display()
            );
            return;
        }

        let path_str = directory_path.to_str();
        if path_str.is_none() {
            error!(
                "Failed to merge glob configurations!\
The path of the sub-directory contains invalid UTF-8 characters.\
Passed in path: {}\
Ignoring new glob configs.",
                directory_path.display()
            );
            return;
        }
        let path_str = path_str.unwrap();

        // Prepend the path to every glob in self and add globs from lower config
        for (glob, _) in self.glob_configs.iter_mut() {
            glob.to_mut().insert_str(0, &(path_str.to_owned() + "/"));
        }

        for (glob, config) in lower.glob_configs {
            self.glob_configs.push((glob, config));
        }
    }
}
