//! This creates a separate test binary so we can modify the environment
//! variables without affecting other tests.

use std::env::remove_var;
use test_binary::{build_test_binary, TestBinaryError};

// Test that the builder returns an error if it's not run under Cargo.
#[test]
fn test_non_cargo_env() {
    remove_var("CARGO");
    remove_var("CARGO_MANIFEST_DIR");
    let result = build_test_binary("does-build", "testbins");
    assert!(matches!(result, Err(TestBinaryError::ManifestError(_))));
}
