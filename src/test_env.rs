#![cfg(any(test, feature = "test-env"))]

use ctor::ctor;
use std::env;

// Initialize deterministic test environment only when building tests or when
// the explicit `test-env` feature is enabled. This file is intentionally
// gated to avoid polluting production binaries with test-only defaults.
#[ctor]
fn init_test_env() {
    // 32 zero bytes base64 (deterministic test key). Important: do NOT leave
    // this value in production or in any CI artifact that will be deployed.
    env::set_var("WALLET_ENC_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    env::set_var("TEST_SKIP_DECRYPT", "1");
    env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");
    env::set_var("BRIDGE_MOCK", "1");
    // Explicitly allow bridge mocks in test builds so startup guard won't block
    env::set_var("ALLOW_BRIDGE_MOCKS", "1");
    tracing::info!("test-env feature active: test env variables set");
}
