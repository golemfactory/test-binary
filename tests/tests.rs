use std::path::PathBuf;
use test_binary::{build_mock_binary, build_mock_binary_with_opts, TestBinaryError};

#[test]
fn test_builds() {
    let result = build_mock_binary("test_it_builds");
    assert!(PathBuf::from(result.unwrap()).ends_with("test_it_builds"));
}

#[test]
fn test_release() {
    let result = build_mock_binary_with_opts("test_it_builds", Some("release"), []);
    assert!(PathBuf::from(result.unwrap()).ends_with("test_it_builds"));
}

#[test]
fn test_doesnt_build() {
    let result = build_mock_binary_with_opts("test_doesnt_build", None, ["test-doesnt-build"]);
    assert!(matches!(result, Err(TestBinaryError::CargoFailure(_))));
}

#[test]
fn test_doesnt_exist() {
    let result = build_mock_binary_with_opts("test_doesnt_build", None, []);
    assert!(matches!(result, Err(TestBinaryError::CargoFailure(_))));
}
