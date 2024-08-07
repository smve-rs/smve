use ignore::Walk;
use serde::Deserialize;
use smve_asset_pack::pack_io::compiling::raw_assets::uncookers::text::TextAssetUncooker;
use smve_asset_pack::pack_io::compiling::raw_assets::AssetUncooker;
use smve_asset_pack::pack_io::compiling::{AssetPackCompiler, CompileResult};
use smve_asset_pack::pack_io::reading::pack_group::AssetPackGroupReader;
use smve_asset_pack::pack_io::reading::AssetPackReader;
use smve_asset_pack::util::text_obfuscation::toggle_obfuscation;
use std::borrow::Cow;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

use test_log::test;

macro_rules! test_out {
    ($fname:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/target/test/", $fname)
    };
}

macro_rules! test_res {
    ($fname:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/resources/", $fname)
    };
}

#[test]
fn full_test() -> Result<(), Box<dyn Error>> {
    compile(Path::new(test_res!("assets_full")))?;
    read()?;

    Ok(())
}

#[test]
fn test_groups() -> Result<(), Box<dyn Error>> {
    let mut reader = AssetPackGroupReader::new(test_res!("asset_group_out"))?;

    reader.register_built_in_pack(
        "builtin",
        Cursor::new(include_bytes!(test_res!("built_in.smap"))),
    )?;

    // Test pack1 overriding pack2
    reader.set_enabled_packs(&["pack1.smap", "pack2.smap"]);
    reader.load()?;
    let mut override_reader = reader.get_file_reader("override.txt")?;
    let mut override_str = String::new();
    override_reader.read_to_string(&mut override_str)?;
    assert_eq!(override_str, "Override1");

    // Test pack1 overriding builtin
    let mut builtin_reader = reader.get_file_reader("builtin.txt")?;
    let mut builtin_str = String::new();
    builtin_reader.read_to_string(&mut builtin_str)?;
    assert_eq!(builtin_str, "Overwritten\n");

    // Test pack2 overriding pack1
    reader.set_enabled_packs(&["pack2.smap", "pack1.smap"]);
    reader.load()?;
    let mut override_reader = reader.get_file_reader("override.txt")?;
    let mut override_str = String::new();
    override_reader.read_to_string(&mut override_str)?;
    assert_eq!(override_str, "Override2");

    // Test singular file that does not get overwritten
    let mut singular_reader = reader.get_file_reader("singular.txt")?;
    let mut singular_str = String::new();
    singular_reader.read_to_string(&mut singular_str)?;
    assert_eq!(singular_str, "Singular");

    // Test builtin overriding pack1
    reader.set_enabled_packs(&["/__built_in/builtin", "pack1.smap", "pack2.smap"]);
    reader.load()?;
    let mut builtin_reader = reader.get_file_reader("builtin.txt")?;
    let mut builtin_str = String::new();
    builtin_reader.read_to_string(&mut builtin_str)?;
    assert_eq!(builtin_str, "BuiltIn\n");

    Ok(())
}

#[derive(Default)]
struct EUncooker;

impl AssetUncooker for EUncooker {
    type Options = EUncookerOptions;

    fn uncook(&self, buf: &[u8], _extension: &str, options: &Self::Options) -> Vec<u8> {
        buf.iter().map(|_| options.character).collect()
    }

    fn target_extension(&self) -> &str {
        "e"
    }

    fn source_extensions(&self) -> &[&str] {
        &["txt"]
    }
}

#[derive(Deserialize)]
struct EUncookerOptions {
    character: u8,
}

impl Default for EUncookerOptions {
    fn default() -> Self {
        Self { character: b'e' }
    }
}

fn setup() -> std::io::Result<()> {
    std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/target/test"))?;

    Ok(())
}

fn compile(assets_path: &Path) -> CompileResult<()> {
    setup()?;

    let out_path = test_out!("out.smap");

    AssetPackCompiler::new()
        .init_asset_uncooker::<TextAssetUncooker>()
        .init_asset_uncooker::<EUncooker>()
        .compile(assets_path, out_path)?;

    Ok(())
}

fn read() -> Result<(), Box<dyn Error>> {
    let out_path = test_out!("out.smap");
    let mut reader = AssetPackReader::new_from_path(out_path)?;

    check_files(Path::new(test_res!("assets_full")), &mut reader)?;

    Ok(())
}

fn check_files(
    dir_path: &Path,
    reader: &mut AssetPackReader<BufReader<File>>,
) -> Result<(), Box<dyn Error>> {
    let walk = Walk::new(dir_path);

    for entry in walk {
        let entry = entry?;

        let rel_path = entry.path().strip_prefix(dir_path).unwrap();
        let mut rel_path_str = Cow::from(rel_path.to_str().unwrap());

        // One entry would be the directory itself, but since we stripped the prefix, it should be
        // an empty string.
        if rel_path_str.is_empty() {
            continue;
        }

        let file_name = entry
            .file_name()
            .to_str()
            .expect("Filename could not be converted to valid UTF-8.");

        // I_ are ignored files, __ indicate asset pack reserved names (__ignore__)
        let not_stored = file_name.starts_with("I_")
            || file_name == "__ignore__"
            || file_name.ends_with("__config__.toml");

        if not_stored {
            assert!(
                !reader.has_file(&rel_path_str)?,
                "Ignored file {rel_path_str} was stored in asset pack!"
            );
            continue;
        }

        if entry.path().is_dir() {
            // Ignore __unique__
            if !rel_path_str.starts_with("__unique__/") && rel_path_str != "__unique__" {
                assert!(
                    reader.has_directory(&rel_path_str)?,
                    "Directory not found in pack: {rel_path_str}"
                );
            }
            continue;
        }

        let e = file_name.starts_with("E_");
        let z = file_name.starts_with("Z_");
        if e || z {
            rel_path_str.to_mut().push_str(".e");
        }

        // nR_ prefix indicates the file is not stored in the raw format
        let raw = !file_name.starts_with("nR_") && !e && !z;
        if raw {
            rel_path_str.to_mut().push_str(".smap_text");
        }

        let mut file_in_pack = if rel_path_str.starts_with("__unique__/") {
            rel_path_str.strip_prefix("__unique__/").unwrap();
            reader.get_unique_file_reader(&rel_path_str)?
        } else {
            reader.get_file_reader(&rel_path_str)?
        };

        let mut data_in_pack = vec![];
        file_in_pack.read_to_end(&mut data_in_pack)?;

        if raw {
            data_in_pack = toggle_obfuscation(&data_in_pack);
        }

        let data_on_disk: Vec<u8> = if e {
            vec![b'e'; 5]
        } else if z {
            vec![b'z'; 5]
        } else {
            let mut file_on_disk = File::open(entry.path())?;
            let mut data_on_disk = vec![];
            file_on_disk.read_to_end(&mut data_on_disk)?;

            data_on_disk
        };

        assert_eq!(
            data_in_pack, data_on_disk,
            "Data stored in pack file does not match data stored on disk for file at {rel_path_str}!"
        );
    }

    Ok(())
}
