//! Thin `wasm-bindgen` binding layer over [`strata_core`].
//!
//! Every method delegates straight to the core; the only work here is string
//! parsing, JSON (de)serialisation at the boundary, and mapping
//! [`strata_core::ArchError`] onto `JsError`.

use std::str::FromStr;

use strata_core::{Architecture, ResourceKind};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// A live architecture document the UI and the WebMCP tools both mutate.
#[wasm_bindgen]
pub struct Studio {
    inner: Architecture,
}

#[wasm_bindgen]
impl Studio {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Studio {
        Studio {
            inner: Architecture::new(),
        }
    }

    /// Add a resource; returns its generated id. Errors on an unknown `kind`.
    #[wasm_bindgen(js_name = addResource)]
    pub fn add_resource(
        &mut self,
        kind: &str,
        label: &str,
        x: f64,
        y: f64,
    ) -> Result<String, JsError> {
        let kind = ResourceKind::from_str(kind).map_err(to_js)?;
        Ok(self.inner.add_resource(kind, label, x, y))
    }

    /// Connect `from -> to`.
    pub fn connect(&mut self, from: &str, to: &str) -> Result<(), JsError> {
        self.inner.connect(from, to).map_err(to_js)
    }

    /// Remove a resource and its incident edges.
    #[wasm_bindgen(js_name = removeResource)]
    pub fn remove_resource(&mut self, id: &str) -> Result<(), JsError> {
        self.inner.remove_resource(id).map_err(to_js)
    }

    /// The full architecture as a JSON string (`{ resources, edges, counters }`).
    #[wasm_bindgen(js_name = stateJson)]
    pub fn state_json(&self) -> String {
        serde_json::to_string(&self.inner).expect("Architecture always serialises")
    }

    /// Replace the whole document (used for undo).
    #[wasm_bindgen(js_name = loadJson)]
    pub fn load_json(&mut self, json: &str) -> Result<(), JsError> {
        self.inner =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("invalid state: {e}")))?;
        Ok(())
    }
}

impl Default for Studio {
    fn default() -> Self {
        Self::new()
    }
}

fn to_js(e: strata_core::ArchError) -> JsError {
    JsError::new(&e.to_string())
}
