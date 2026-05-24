use anyhow::{anyhow, Result};
use mlua::{Function, Lua, Result as LuaResult, Table};
use std::path::Path;

pub struct LuaSlideshowScript {
    lua: Lua,
    has_on_advance: bool,
    has_on_interval: bool,
}

#[derive(Debug, Clone)]
pub struct SlideContext {
    pub current_index: usize,
    pub total: usize,
    pub interval_secs: f64,
    pub elapsed_secs: f64,
}

#[derive(Debug, Default)]
pub struct SlideCommand {
    pub next_index: Option<usize>,
    pub new_interval: Option<f64>,
    pub zoom_target: Option<f32>,
    pub zoom_duration: Option<f32>,
}

impl LuaSlideshowScript {
    pub fn load(path: &Path) -> Result<Self> {
        let source = std::fs::read_to_string(path)?;
        Self::from_str(&source)
    }

    pub fn from_str(source: &str) -> Result<Self> {
        if source.trim().is_empty() {
            return Err(anyhow!("empty script"));
        }
        let lua = Lua::new();
        lua.load(source).exec().map_err(|e| anyhow!("Lua error: {e}"))?;

        let has_on_advance = lua.globals().get::<Function>("on_advance").is_ok();
        let has_on_interval = lua.globals().get::<Function>("on_interval").is_ok();

        Ok(Self { lua, has_on_advance, has_on_interval })
    }

    pub fn on_advance(&self, ctx: &SlideContext) -> Result<SlideCommand> {
        if !self.has_on_advance {
            return Ok(SlideCommand::default());
        }
        let func: Function = self.lua.globals()
            .get("on_advance")
            .map_err(|e| anyhow!("Lua: {e}"))?;
        let table = self.lua.create_table().map_err(|e| anyhow!("Lua: {e}"))?;
        fill_table(&table, ctx).map_err(|e| anyhow!("Lua: {e}"))?;
        let result: Table = func.call(table).map_err(|e| anyhow!("Lua on_advance: {e}"))?;
        Ok(parse_command(&result))
    }

    pub fn on_interval(&self, ctx: &SlideContext) -> Result<SlideCommand> {
        if !self.has_on_interval {
            return Ok(SlideCommand::default());
        }
        let func: Function = self.lua.globals()
            .get("on_interval")
            .map_err(|e| anyhow!("Lua: {e}"))?;
        let table = self.lua.create_table().map_err(|e| anyhow!("Lua: {e}"))?;
        fill_table(&table, ctx).map_err(|e| anyhow!("Lua: {e}"))?;
        let result: Table = func.call(table).map_err(|e| anyhow!("Lua on_interval: {e}"))?;
        Ok(parse_command(&result))
    }
}

fn fill_table(table: &Table, ctx: &SlideContext) -> LuaResult<()> {
    table.set("current_index", ctx.current_index)?;
    table.set("total", ctx.total)?;
    table.set("interval_secs", ctx.interval_secs)?;
    table.set("elapsed_secs", ctx.elapsed_secs)?;
    Ok(())
}

fn parse_command(table: &Table) -> SlideCommand {
    SlideCommand {
        next_index: table.get("next_index").ok(),
        new_interval: table.get("new_interval").ok(),
        zoom_target: table.get::<f64>("zoom_target").ok().map(|v| v as f32),
        zoom_duration: table.get::<f64>("zoom_duration").ok().map(|v| v as f32),
    }
}
