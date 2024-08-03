pub mod glob_utils;
mod merge_utils;

use log::{error, warn};
use merge::Merge;
use serde::Deserialize;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml::Table;

#[derive(Deserialize, Clone, Merge)]
pub struct DirectoryConfiguration<'a> {
    #[serde(flatten)]
    #[merge(strategy = merge::option::recurse)]
    pub dir_config: Option<Configuration<'a>>,
    #[serde(flatten, borrow, with = "tuple_vec_map")]
    #[merge(strategy = merge::vec::append)]
    pub glob_configs: Vec<(Cow<'a, str>, Configuration<'a>)>,
}

impl Default for DirectoryConfiguration<'_> {
    fn default() -> Self {
        Self {
            dir_config: Some(Configuration::default()),
            glob_configs: Vec::default(),
        }
    }
}

#[derive(Deserialize, Clone, Merge)]
#[serde(default)]
pub struct Configuration<'a> {
    #[merge(strategy = merge::option::recurse)]
    pub compression: Option<CompressionOptions>,
    #[merge(strategy = merge::option::recurse)]
    #[serde(borrow)]
    pub uncooker: Option<UncookerOptions<'a>>,
}

impl Default for Configuration<'_> {
    fn default() -> Self {
        Self {
            compression: Some(CompressionOptions::default()),
            uncooker: Some(UncookerOptions::default()),
        }
    }
}

impl Configuration<'_> {
    pub fn empty() -> Self {
        Self {
            compression: None,
            uncooker: None,
        }
    }
}

#[derive(Deserialize, Clone, Merge)]
#[serde(default)]
pub struct CompressionOptions {
    #[merge(strategy = merge::option::overwrite_none)]
    pub enabled: Option<bool>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub level: Option<u8>,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            level: Some(4),
        }
    }
}

#[derive(Deserialize, Clone, Merge)]
#[serde(default)]
pub struct UncookerOptions<'a> {
    #[merge(strategy = merge::option::overwrite_none)]
    pub enabled: Option<bool>,
    #[merge(strategy = merge::option::overwrite_none)]
    #[serde(borrow)]
    pub uncooker_path: Option<Cow<'a, str>>,
    #[serde(flatten)]
    #[merge(strategy = merge_utils::merge_option_table)]
    pub options: Option<Table>,
}

impl Default for UncookerOptions<'_> {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            uncooker_path: None,
            options: Some(Table::default()),
        }
    }
}

pub fn get_dir_config<'de>(dir: impl AsRef<Path>) -> Option<DirectoryConfiguration<'de>> {
    let config_path = dir.as_ref().join("__config__.toml");

    let table = get_config(&config_path)?;

    let configs: Result<DirectoryConfiguration, _> = table.try_into();

    match configs {
        Ok(mut config) => {
            let path_string = dir.as_ref().to_str();

            if path_string.is_none() {
                warn!(
                    "Directory {} contains invalid UTF-8 characters, removing all glob configs.",
                    dir.as_ref().display()
                );

                config.glob_configs = vec![];

                return Some(config);
            }

            for (ref mut path, _) in &mut config.glob_configs {
                path.to_mut()
                    .insert_str(0, &format!("{}/", path_string.unwrap()));
            }

            Some(config)
        }
        Err(error) => {
            error!("Failed to interpret config file at {} because the structure of the config file is incorrect. From TOML error: {error}", config_path.display());
            None
        }
    }
}

pub fn get_file_config<'de>(file_path: impl AsRef<Path>) -> Option<Configuration<'de>> {
    let path = file_path.as_ref();

    let mut path_osstr = path.as_os_str().to_os_string();
    //      filename.ext.__config__.toml
    path_osstr.push(".__config__.toml");

    let config_path = Path::new(&path_osstr);

    let table = get_config(config_path)?;

    let config: Result<Configuration, _> = table.try_into();

    if let Err(error) = config {
        error!("Failed to interpret config file at {} because the structure of the config file is incorrect. From TOML error: {error}", config_path.display());
        None
    } else {
        config.ok()
    }
}

fn get_config(config_path: &Path) -> Option<Table> {
    if config_path.exists() && config_path.is_file() {
        let config_file = File::open(config_path);
        if let Err(error) = config_file {
            error!("Failed to open config file at {}, ignoring config for this directory. IO error: {error}", config_path.display());
            return None;
        }

        let mut config_file = config_file.unwrap();

        let mut file_string = String::new();
        let read_result = config_file.read_to_string(&mut file_string);
        if let Err(error) = read_result {
            error!("Failed to read config file at {}, ignoring config for this directory. IO error: {error}", config_path.display());
            return None;
        }

        let config: Result<Table, _> = toml::from_str(&file_string);
        if let Err(error) = &config {
            error!("Failed to parse config file at {}, ignoring config for this directory. DE error: {error}", config_path.display());
        }

        Some(config.unwrap())
    } else {
        None
    }
}
