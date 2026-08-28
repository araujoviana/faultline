//! Runs the real WASM in a headless browser via `wasm-pack test --headless --chrome`.
//! Skipped on native `cargo test`.
#![cfg(target_arch = "wasm32")]

use strata_wasm::Studio;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn add_resource_shows_up_in_state() {
    let mut studio = Studio::new();
    let id = studio
        .add_resource("database", "orders db", 12.0, 34.0)
        .unwrap();
    assert_eq!(id, "database-1");

    let state = studio.state_json();
    assert!(state.contains("orders db"));
    assert!(state.contains("database-1"));
}

#[wasm_bindgen_test]
fn unknown_kind_is_an_error() {
    let mut studio = Studio::new();
    assert!(studio.add_resource("nope", "x", 0.0, 0.0).is_err());
}
