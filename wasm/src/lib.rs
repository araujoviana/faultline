//! Thin `wasm-bindgen` binding layer over [`strata_core`].
//!
//! Every method delegates straight to the core; the only work here is string
//! parsing, JSON (de)serialisation at the boundary, and mapping
//! [`strata_core::ArchError`] onto `JsError`.

use std::str::FromStr;

use strata_core::analysis::{az_failure_seed, blast_radius, spofs};
use strata_core::cost::estimate as run_estimate;
use strata_core::explain::explain as run_explain;
use strata_core::lint::lint as run_lint;
use strata_core::profile::ProviderProfile;
use strata_core::propose::propose as run_propose;
use strata_core::{iac, Architecture, ResourceKind};
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
    profile: ProviderProfile,
}

#[wasm_bindgen]
impl Studio {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Studio {
        Studio {
            inner: Architecture::new(),
            profile: ProviderProfile::aws(),
        }
    }

    /// Replace the whole design with a starting architecture built from a
    /// free-text requirements sentence (deterministic keyword matching).
    #[wasm_bindgen(js_name = propose)]
    pub fn propose(&mut self, requirements: &str) {
        self.inner = run_propose(requirements);
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

    /// Move a resource to a new canvas position.
    #[wasm_bindgen(js_name = moveResource)]
    pub fn move_resource(&mut self, id: &str, x: f64, y: f64) -> Result<(), JsError> {
        self.inner.move_resource(id, x, y).map_err(to_js)
    }

    /// Remove a resource and its incident edges.
    #[wasm_bindgen(js_name = removeResource)]
    pub fn remove_resource(&mut self, id: &str) -> Result<(), JsError> {
        self.inner.remove_resource(id).map_err(to_js)
    }

    /// Set a resource's provider variant and/or placement.
    ///
    /// - `variant` empty → variant left unchanged; otherwise validated against
    ///   the active profile.
    /// - `region` empty → placement left unchanged.
    /// - `region` set, `az` empty → regional (any prior zone is cleared).
    /// - `region` and `az` set → zonal.
    pub fn configure(
        &mut self,
        id: &str,
        variant: Option<String>,
        region: Option<String>,
        az: Option<String>,
    ) -> Result<(), JsError> {
        let resource = self
            .inner
            .resources
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| JsError::new(&format!("no such resource: {id}")))?;
        let kind = resource.kind;

        if let Some(variant) = variant.filter(|v| !v.is_empty()) {
            if self.profile.variant(kind, &variant).is_none() {
                return Err(JsError::new(&format!(
                    "unknown {kind} variant for {}: {variant}",
                    self.profile.display_name
                )));
            }
            self.inner.set_variant(id, Some(variant)).map_err(to_js)?;
        }

        if let Some(region) = region.filter(|r| !r.is_empty()) {
            let az = az.filter(|a| !a.is_empty());
            if let Some(az) = &az {
                if !self.profile.has_az(&region, az) {
                    return Err(JsError::new(&format!(
                        "{az} is not an availability zone of {region}"
                    )));
                }
            } else if self.profile.region(&region).is_none() {
                return Err(JsError::new(&format!("unknown region: {region}")));
            }
            self.inner.place(id, Some(region), az).map_err(to_js)?;
        }

        Ok(())
    }

    /// Simulate the loss of availability zone `az` in `region`. Returns a
    /// `BlastReport` as JSON.
    #[wasm_bindgen(js_name = simulateFailure)]
    pub fn simulate_failure(&self, region: &str, az: &str) -> Result<String, JsError> {
        if !self.profile.has_az(region, az) {
            return Err(JsError::new(&format!(
                "{az} is not an availability zone of {region}"
            )));
        }
        let seed = az_failure_seed(&self.inner, az);
        let report = blast_radius(
            &self.inner,
            &self.profile,
            &seed,
            Some(region),
            &format!("AZ {az}"),
        );
        Ok(serde_json::to_string(&report).expect("BlastReport always serialises"))
    }

    /// Single points of failure in the current design, as a JSON `Spof[]`.
    #[wasm_bindgen(js_name = findSpofs)]
    pub fn find_spofs(&self) -> String {
        let found = spofs(&self.inner, &self.profile);
        serde_json::to_string(&found).expect("Spof list always serialises")
    }

    /// Resilience-lint the current design: rule-based architectural findings,
    /// each citing a principle from *Designing Data-Intensive Applications*.
    /// Read-only; returns a JSON `Finding[]`.
    #[wasm_bindgen(js_name = lint)]
    pub fn lint(&self) -> String {
        let found = run_lint(&self.inner, &self.profile);
        serde_json::to_string(&found).expect("Finding list always serialises")
    }

    /// Rough monthly cost estimate for the current design, as a JSON
    /// `CostReport`. Read-only; a bundled-snapshot figure, not a live quote.
    #[wasm_bindgen(js_name = estimateCost)]
    pub fn estimate_cost(&self) -> String {
        let report = run_estimate(&self.inner, &self.profile);
        serde_json::to_string(&report).expect("CostReport always serialises")
    }

    /// Explain one selection — a resource id, or an edge written `"from->to"`.
    /// Read-only; returns an `Explanation` as JSON.
    #[wasm_bindgen(js_name = explain)]
    pub fn explain(&self, selection: &str) -> String {
        let explanation = run_explain(&self.inner, &self.profile, selection);
        serde_json::to_string(&explanation).expect("Explanation always serialises")
    }

    /// The active provider profile as JSON (service catalogue + region topology).
    #[wasm_bindgen(js_name = profileJson)]
    pub fn profile_json(&self) -> String {
        serde_json::to_string(&self.profile).expect("ProviderProfile always serialises")
    }

    /// Emit the current architecture as infrastructure-as-code (read-only, no
    /// mutation). Only `"terraform"` (or `""` for the default) is supported.
    #[wasm_bindgen(js_name = generateIac)]
    pub fn generate_iac(&self, target: &str) -> Result<String, JsError> {
        match target {
            "" | "terraform" => Ok(iac::emit_terraform(&self.inner, &self.profile)),
            other => Err(JsError::new(&format!(
                "unknown target: {other}. Supported: terraform"
            ))),
        }
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
