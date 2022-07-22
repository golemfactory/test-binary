//! Integration tests for mock binary builds.

use std::path::PathBuf;
use test_binary::{
    build_mock_binary, build_mock_binary_once, build_mock_binary_with_opts, TestBinaryError,
};

// Singleton function for "test_multiple" binary.
build_mock_binary_once!(test_multiple);

/// Test that a binary which should build, does build.
#[test]
fn test_builds() {
    let result = build_mock_binary("test_it_builds");
    assert!(PathBuf::from(result.unwrap()).ends_with("test_it_builds"));
}

/// Test building a binary with a non-default profile (release).
#[test]
fn test_release() {
    let result = build_mock_binary_with_opts("test_it_builds", Some("release"), []);
    assert!(PathBuf::from(result.unwrap()).ends_with("test_it_builds"));
}

/// Test that building a binary that doesn't build produces an error.
#[test]
fn test_doesnt_build() {
    let result = build_mock_binary("test_doesnt_build");
    assert!(matches!(result, Err(TestBinaryError::BuildError(_))));
}

/// Test that building a binary that doesn't exist produces an error. Note that
/// there is no (stable, reliable) way to distinguish errors above the level of
/// build failures, because they don't appear in the JSON output but rather, as
/// prose on stderr.
#[test]
fn test_doesnt_exist() {
    let result = build_mock_binary_with_opts("test_doesnt_exist", None, []);
    assert!(matches!(result, Err(TestBinaryError::CargoFailure(_))));
}

/// Test calling the macro generated build function. Note that the
/// `test_multiple_calls_x()` functions do not test laziness, mutual exclusion
/// or timing, but they act as a check against the macro failing to do its job.
#[test]
fn test_multiple_calls_1() {
    let result = test_multiple_path();
    assert!(PathBuf::from(result).ends_with("test_multiple"));
}

#[test]
fn test_multiple_calls_2() {
    let result = test_multiple_path();
    assert!(PathBuf::from(result).ends_with("test_multiple"));
}

#[test]
fn test_multiple_calls_3() {
    let result = test_multiple_path();
    assert!(PathBuf::from(result).ends_with("test_multiple"));
}

#[test]
fn test_multiple_calls_4() {
    let result = test_multiple_path();
    assert!(PathBuf::from(result).ends_with("test_multiple"));
}
