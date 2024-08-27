//! Contains an uncooker type that wraps a lua uncooker.

use mlua::{Function, Lua, LuaSerdeExt, RegistryKey, Table, Value};
use smve_asset_pack::pack_io::compiling::raw_assets::AssetUncooker;

/// An uncooker type that wraps an uncooker defined in a lua file.
pub struct UserDefinedUncooker {
    lua: Lua,
    target_extension: String,
    source_extensions: Vec<String>,
    default_config: RegistryKey,
}

impl UserDefinedUncooker {
    /// Creates a new uncooker from the lua string.
    pub fn new(lua_str: &str) -> mlua::Result<Self> {
        let lua = Lua::new();

        let chunk = lua.load(lua_str);

        let empty_table = lua.create_table()?;
        let chunk = chunk.set_environment(&empty_table);
        chunk.exec()?;

        let globals = lua.globals();

        empty_table.for_each(|key: String, value: Value| {
            if !globals.contains_key(key.as_str())? {
                globals.raw_set(key, value)?;
            }

            Ok(())
        })?;

        drop(empty_table);

        let _uncook_function: Function = globals.get("Uncook")?;
        let target_extension = globals.get("TARGET_EXTENSION")?;
        let source_extensions = globals.get("SOURCE_EXTENSIONS")?;
        let default_config: Table = globals.get("DEFAULT_CONFIG")?;

        let default_config = lua.create_registry_value(default_config)?;

        drop(globals);
        drop(_uncook_function);

        let this = Self {
            lua,
            target_extension,
            source_extensions,
            default_config,
        };

        Ok(this)
    }
}

impl AssetUncooker for UserDefinedUncooker {
    type Options = toml::Table;
    type Error = mlua::Error;

    fn uncook(
        &self,
        buf: &[u8],
        extension: &str,
        options: &Self::Options,
    ) -> Result<Vec<u8>, Self::Error> {
        let uncook: Function = self.lua.globals().get("Uncook")?;
        uncook.set_environment(self.lua.globals())?;

        let options = if options.is_empty() {
            self.lua.registry_value(&self.default_config)?
        } else {
            self.lua.to_value(options)?
        };

        uncook.call::<_, Vec<u8>>((buf, extension, options))
    }

    fn target_extension(&self) -> &str {
        &self.target_extension
    }

    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.source_extensions.iter().map(|s| s.as_str()))
    }
}
