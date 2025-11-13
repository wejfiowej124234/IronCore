// This stub exists because some CI runs reference `tests/tools_serdes_tests.rs`.
// It contains a no-op test so the test binary can be created if cargo expects this path.
// Updated to trigger CI rerun.

#[test]
fn ci_tools_serdes_stub() {
    // intentionally empty; real serdes tests live in `serdes_tests.rs`.
}
