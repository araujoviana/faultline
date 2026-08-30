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

#[wasm_bindgen_test]
fn move_resource_updates_position_in_state() {
    let mut studio = Studio::new();
    let id = studio.add_resource("compute", "api", 0.0, 0.0).unwrap();
    studio.move_resource(&id, 128.0, 256.0).unwrap();
    let state = studio.state_json();
    assert!(state.contains("\"x\":128"));
    assert!(state.contains("\"y\":256"));
    assert!(studio.move_resource("ghost-1", 1.0, 1.0).is_err());
}

#[wasm_bindgen_test]
fn az_failure_blast_radius_runs_on_the_real_wasm() {
    let mut studio = Studio::new();
    let lb = studio
        .add_resource("load-balancer", "edge", 0.0, 0.0)
        .unwrap();
    let api = studio.add_resource("compute", "api", 0.0, 0.0).unwrap();
    let db = studio.add_resource("database", "orders", 0.0, 0.0).unwrap();
    studio.connect(&lb, &api).unwrap();
    studio.connect(&api, &db).unwrap();
    studio
        .configure(
            &db,
            Some("rds-single-az".into()),
            Some("us-east-1".into()),
            Some("us-east-1a".into()),
        )
        .unwrap();

    let report = studio.simulate_failure("us-east-1", "us-east-1a").unwrap();
    assert!(report.contains("database-1"));
    assert!(report.contains("compute-1"));
    assert!(report.contains("load-balancer-1"));

    let spofs = studio.find_spofs();
    assert!(spofs.contains("database-1"));

    // Swapping to Multi-AZ turns the outage into a degradation.
    studio
        .configure(
            &db,
            Some("rds-multi-az".into()),
            Some("us-east-1".into()),
            None,
        )
        .unwrap();
    let report = studio.simulate_failure("us-east-1", "us-east-1a").unwrap();
    assert!(report.contains("\"down\":[]"));
    assert!(report.contains("fail over"));
    assert!(studio.find_spofs().contains("[]"));
}

#[wasm_bindgen_test]
fn generate_iac_runs_on_the_real_wasm() {
    let mut studio = Studio::new();
    let lb = studio
        .add_resource("load-balancer", "edge", 0.0, 0.0)
        .unwrap();
    let api = studio.add_resource("compute", "api", 0.0, 0.0).unwrap();
    let db = studio.add_resource("database", "orders", 0.0, 0.0).unwrap();
    studio.connect(&lb, &api).unwrap();
    studio.connect(&api, &db).unwrap();
    studio
        .configure(&lb, Some("alb".into()), None, None)
        .unwrap();
    studio
        .configure(&api, Some("ec2-asg".into()), None, None)
        .unwrap();
    studio
        .configure(
            &db,
            Some("rds-multi-az".into()),
            Some("us-east-1".into()),
            None,
        )
        .unwrap();

    let hcl = studio.generate_iac("terraform").unwrap();
    assert!(hcl.contains("terraform {"));
    assert!(hcl.contains("resource \"aws_db_instance\" \"database_1\""));
    assert!(hcl.contains("multi_az"));
    assert!(hcl.contains("target_group_arns"));

    assert!(studio.generate_iac("pulumi").is_err());
}

#[wasm_bindgen_test]
fn new_service_kinds_add_and_emit() {
    let mut studio = Studio::new();
    for kind in ["cdn", "dns", "functions", "api-gateway"] {
        assert!(studio.add_resource(kind, kind, 0.0, 0.0).is_ok());
    }
    studio
        .configure("functions-1", Some("lambda".into()), None, None)
        .unwrap();
    let hcl = studio.generate_iac("").unwrap();
    assert!(hcl.contains("resource \"aws_lambda_function\" \"functions_1\""));
}
