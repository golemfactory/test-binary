//! Integration tests for mock binary builds.

use std::path::{Path, PathBuf};
use test_binary::{build_test_binary, build_test_binary_once, TestBinary, TestBinaryError};

// Singleton function for "test_multiple" binary.
build_test_binary_once!(multiple, "testbins");

fn assert_path_end<R: AsRef<Path>>(actual: R, expected_ending: &str) {
    assert!(actual.as_ref().ends_with(expected_ending))
}

/// Test that a binary which should build, does build.
#[test]
fn test_builds() {
    let result = build_test_binary("does-build", "testbins");
    assert_path_end(result.unwrap(), "does-build");
}

/// Test building a binary with a non-default profile (release).
#[test]
fn test_release() {
    let result = TestBinary::relative_to_parent(
        "does-build",
        &PathBuf::from_iter(["testbins", "does-build", "Cargo.toml"]),
    )
    .with_profile("release")
    .build();

    assert_path_end(result.unwrap(), "does-build");
}

/// Test that building a binary that doesn't build produces an error.
#[test]
fn test_doesnt_build() {
    let result = build_test_binary("doesnt-build", "testbins");
    assert!(matches!(result, Err(TestBinaryError::BuildError)));
}

/// Test that building a binary that doesn't exist produces an error. Note that
/// there is no (stable, reliable) way to distinguish errors above the level of
/// build failures, because they don't appear in the JSON output but rather, as
/// prose on stderr.
#[test]
fn test_doesnt_exist() {
    let result = build_test_binary("doesnt-exist", "testbins");
    assert!(matches!(result, Err(TestBinaryError::CargoFailure(_))));
}

/// Test calling the macro generated build function. Note that the
/// `test_multiple_calls_x()` functions do not test laziness, mutual exclusion
/// or timing, but they act as a check against the macro failing to do its job.
#[test]
fn test_multiple_calls_1() {
    let result = path_to_multiple();
    assert_path_end(result, "multiple");
}

#[test]
fn test_multiple_calls_2() {
    let result = path_to_multiple();
    assert_path_end(result, "multiple");
}

#[test]
fn test_multiple_calls_3() {
    let result = path_to_multiple();
    assert_path_end(result, "multiple");
}

#[test]
fn test_multiple_calls_4() {
    let result = path_to_multiple();
    assert_path_end(result, "multiple");
}
