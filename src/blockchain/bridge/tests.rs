// filepath: src/blockchain/bridge/tests.rs
use super::relay::{bridge_mocks_allowed, bridge_mocks_requested_truthy};
use std::env;

#[test]
fn test_bridge_mock_gating_envs() {
    // Save and clear relevant envs
    let keys = [
        "ALLOW_BRIDGE_MOCKS",
        "BRIDGE_MOCK_FORCE_SUCCESS",
        "BRIDGE_MOCK",
        "FORCE_BRIDGE_SUCCESS",
        "BRIDGE_MOCK_FORCE",
    ];
    let saved: Vec<(String, Option<String>)> = keys
        .iter()
        .map(|k| (k.to_string(), env::var(k).ok()))
        .collect();
    for k in &keys {
        env::remove_var(k);
    }

    // When nothing is set, requested = false, allowed = (false unless test-env feature set)
    assert_eq!(bridge_mocks_requested_truthy(), false);
    if !cfg!(feature = "test-env") {
        assert_eq!(bridge_mocks_allowed(), false);
    }

    // Request mocks but do not allow -> requested true, allowed false
    env::set_var("BRIDGE_MOCK", "1");
    assert_eq!(bridge_mocks_requested_truthy(), true);
    if !cfg!(feature = "test-env") {
        assert_eq!(bridge_mocks_allowed(), false);
    }

    // Allow mocks explicitly -> allowed true
    env::set_var("ALLOW_BRIDGE_MOCKS", "1");
    assert_eq!(bridge_mocks_allowed(), true);

    // Cleanup: restore envs
    for (k, v) in saved {
        match v {
            Some(val) => env::set_var(k, val),
            None => env::remove_var(k),
        }
    }
}