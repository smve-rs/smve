//! Contains an uncooker type that wraps a lua uncooker.

use std::fmt::Display;

use mlua::{Function, Lua, LuaSerdeExt, RegistryKey, Table, Value};
use smve_asset_pack::pack_io::compiling::raw_assets::AssetUncooker;
use snafu::{Location, Snafu};

macro_rules! uncook {
    ($expr:expr, $step:expr) => {{
        use snafu::ResultExt;
        $expr.with_context(|_| UncookerCtx { step: $step })
    }};
}

/// An uncooker type that wraps an uncooker defined in a lua file.
pub struct UserDefinedUncooker {
    lua: Lua,
    target_extension: String,
    source_extensions: Vec<String>,
    default_config: RegistryKey,
}

impl UserDefinedUncooker {
    /// Creates a new uncooker from the lua string.
    pub fn new(lua_str: &str) -> Result<Self, UncookerError> {
        let lua = Lua::new();

        let chunk = lua.load(lua_str);

        let empty_table = uncook!(lua.create_table(), UncookerStep::Initialize)?;
        let chunk = chunk.set_environment(&empty_table);
        uncook!(chunk.exec(), UncookerStep::Initialize)?;

        let globals = lua.globals();

        uncook!(
            empty_table.for_each(|key: String, value: Value<'_>| {
                if !globals.contains_key(key.as_str())? {
                    globals.raw_set(key, value)?;
                }

                Ok(())
            }),
            UncookerStep::Initialize
        )?;

        drop(empty_table);

        let _uncook_function: Function<'_> =
            uncook!(globals.get("Uncook"), UncookerStep::GetGlobals)?;
        let target_extension = uncook!(globals.get("TARGET_EXTENSION"), UncookerStep::GetGlobals)?;
        let source_extensions =
            uncook!(globals.get("SOURCE_EXTENSIONS"), UncookerStep::GetGlobals)?;
        let default_config: Table<'_> =
            uncook!(globals.get("DEFAULT_CONFIG"), UncookerStep::GetGlobals)?;

        let default_config = uncook!(
            lua.create_registry_value(default_config),
            UncookerStep::Initialize
        )?;

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
    type Error = UncookerError;

    fn uncook(
        &self,
        buf: &[u8],
        extension: &str,
        options: &Self::Options,
    ) -> Result<Vec<u8>, Self::Error> {
        let uncook: Function<'_> =
            uncook!(self.lua.globals().get("Uncook"), UncookerStep::RunUncook)?;
        uncook!(
            uncook.set_environment(self.lua.globals()),
            UncookerStep::RunUncook
        )?;

        let options = if options.is_empty() {
            uncook!(
                self.lua.registry_value(&self.default_config),
                UncookerStep::RunUncook
            )?
        } else {
            uncook!(self.lua.to_value(options), UncookerStep::RunUncook)?
        };

        uncook!(
            uncook.call::<_, Vec<u8>>((buf, extension, options)),
            UncookerStep::RunUncook
        )
    }

    fn target_extension(&self) -> &str {
        &self.target_extension
    }

    fn source_extensions(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.source_extensions.iter().map(|s| s.as_str()))
    }
}

#[derive(Snafu, Debug)]
#[snafu(
    context(suffix(Ctx)),
    display("Encountered Lua error while {step} at {location}: {source}")
)]
/// A wrapper around lua error.
pub struct UncookerError {
    source: mlua::Error,
    step: UncookerStep,
    #[snafu(implicit)]
    location: Location,
}

#[derive(Debug)]
/// Represents various places where a lua error can be encountered.
pub enum UncookerStep {
    /// Initializing lua runtime
    Initialize,
    /// Getting predefined globals
    GetGlobals,
    /// Running the uncook function
    RunUncook,
}

impl Display for UncookerStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UncookerStep::Initialize => write!(f, "initializing lua runtime"),
            UncookerStep::GetGlobals => write!(f, "getting globals"),
            UncookerStep::RunUncook => write!(f, "running uncook function"),
        }
    }
}
