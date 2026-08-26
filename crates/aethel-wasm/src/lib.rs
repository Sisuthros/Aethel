//! WebAssembly sandbox execution for Aethel (NG4).
//!
//! This crate is no longer a stub: it can compile and execute real Wasm
//! modules via wasmtime 45. Guest code runs inside a fresh, empty store
//! so host authority is not exposed by default.

use anyhow::{bail, Result};
use wasmtime::{Engine, FuncType, Instance, Module, Store, Val, ValType};

/// A sandboxed WebAssembly runtime.
///
/// Holds a reusable wasmtime [`Engine`]. Each execution uses its own
/// [`Store`] so guest state is isolated.
#[derive(Debug, Clone)]
pub struct WasmRuntime {
    engine: Engine,
}

impl WasmRuntime {
    /// Create a new runtime with a default wasmtime engine.
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        Ok(Self { engine })
    }

    /// Compile a textual WebAssembly module (WAT) and call one exported
    /// function with integer arguments.
    ///
    /// Returns the first integer result. This is intentionally narrow:
    /// it is enough to prove that real Wasm execution works end-to-end
    /// without building a full ABI for Aethel IR yet.
    ///
    /// # Errors
    ///
    /// - `func` is not an export of the module.
    /// - The function signature does not accept the provided arguments.
    /// - The guest traps or returns a non-integer result.
    pub fn call_i32(&self, wat: &str, func: &str, args: &[i32]) -> Result<i32> {
        let module = Module::new(&self.engine, wat)
            .map_err(|e| anyhow::anyhow!("failed to compile guest WAT module: {e}"))?;

        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| anyhow::anyhow!("failed to instantiate guest module: {e}"))?;

        let export = instance
            .get_export(&mut store, func)
            .ok_or_else(|| anyhow::anyhow!("guest module has no export `{func}`"))?;
        let guest = export
            .into_func()
            .ok_or_else(|| anyhow::anyhow!("guest export `{func}` is not a function"))?;

        let param_types: Vec<ValType> = args.iter().map(|_| ValType::I32).collect();
        let expected = FuncType::new(
            &self.engine,
            param_types.clone(),
            [ValType::I32].iter().cloned(),
        );
        let actual = guest.ty(&store);
        if !Self::matches_narrowly(&expected, &actual) {
            bail!(
                "guest function `{func}` signature mismatch: expected {:?}, got {:?}",
                expected,
                actual
            );
        }

        let wasm_args: Vec<Val> = args.iter().map(|v| Val::I32(*v)).collect();
        let mut results = [Val::I32(0)];
        guest
            .call(&mut store, &wasm_args, &mut results)
            .map_err(|e| anyhow::anyhow!("guest function `{func}` trapped: {e}"))?;

        match results[0] {
            Val::I32(value) => Ok(value),
            other => bail!("guest function `{func}` returned non-i32: {:?}", other),
        }
    }

    /// Returns true if the actual function type has the requested parameter
    /// count and at least one i32 result. We keep the check narrow because the
    /// caller is a test/proof-of-concept ABI, not a general host binding.
    fn matches_narrowly(expected: &FuncType, actual: &FuncType) -> bool {
        expected.params().len() == actual.params().len()
            && expected
                .params()
                .zip(actual.params())
                .all(|(a, b)| matches!((a, b), (ValType::I32, ValType::I32)))
            && actual.results().len() >= 1
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("default wasmtime engine should always build")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADD_WAT: &str = r#"
        (module
          (func (export "add") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add)
        )
    "#;

    const FACTORIAL_WAT: &str = r#"
        (module
          (func (export "fac") (param i32) (result i32)
            local.get 0
            i32.eqz
            if (result i32)
              i32.const 1
            else
              local.get 0
              local.get 0
              i32.const 1
              i32.sub
              call 0
              i32.mul
            end)
        )
    "#;

    #[test]
    fn compiles_and_runs_a_guest_function() {
        let rt = WasmRuntime::new().unwrap();
        let result = rt.call_i32(ADD_WAT, "add", &[7, 8]).unwrap();
        assert_eq!(result, 15);
    }

    #[test]
    fn traps_on_missing_export() {
        let rt = WasmRuntime::new().unwrap();
        let err = rt.call_i32(ADD_WAT, "missing", &[1, 2]).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("has no export `missing`"),
            "unexpected message: {}",
            msg
        );
    }

    #[test]
    fn recursive_guest_function_works() {
        let rt = WasmRuntime::new().unwrap();
        let result = rt.call_i32(FACTORIAL_WAT, "fac", &[5]).unwrap();
        assert_eq!(result, 120);
    }
}
