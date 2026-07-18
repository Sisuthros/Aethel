//! WASM execution (stub).

use aethel_ir::lower::IrModule;
use anyhow::Result;

pub struct WasmRuntime;

impl WasmRuntime {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn execute(&self, _module: &IrModule) -> Result<()> {
        Ok(())
    }
}
