//! Test binary generation for integration tests under Cargo.
//!
//! If you have integration tests for things that involve subprocess management,
//! inter-process communication, or platform tools, you might need to write some
//! mock binaries of your own to test against. And if you're already using Cargo
//! to build and test, it would be nice to be able to write those test binaries
//! in Rust, in the crate you're testing, as crate binaries themselves.
//!
//! This crate provides a simple interface for invoking Cargo to build test
//! binaries in your own crate, defined in your `Cargo.toml`. Call
//! [`build_mock_binary("name_of_binary")`] where `"name_of_binary"` is the
//! binary name you'd pass to Cargo eg. `cargo build --bin name_of_binary`. If
//! you need to change profiles or features, there is
//! [`build_mock_binary_with_opts()`].
//!
//! Here's an example of how you might use this in a test, with a binary named
//! `test_it_builds`
//!
//! ```rust
//! # use test_binary::build_mock_binary;
//! let test_bin_path = build_mock_binary("test_it_builds").expect("Error building test binary");
//! let mut test_bin_subproc = std::process::Command::new(test_bin_path)
//!     .spawn()
//!     .expect("Error running test binary");
//!
//! // Test behaviour of your program against the mock binary eg. send it
//! // something on stdin and assert what it prints on stdout, do some IPC,
//! // check for side effects.
//!
//! assert!(test_bin_subproc
//!     .wait()
//!     .expect("Error waiting for test binary")
//!     .success());
//! ```
//!
//! The result returned by these functions contains the path of the built binary
//! as a [`std::ffi::OsString`], which can be passed to
//! [`std::process::Command`] or other crates that deal with subprocesses. The
//! path is not resolved to an absolute path, although it might be one anyway.
//! Since it is the path provided by Cargo after being invoked in the current
//! process' working directory, it will be valid as long as you do not change
//! the working directory between obtaining it and using it.

use cargo_metadata::Message;
use std::ffi::OsString;
use std::io::{BufReader, Read};
use std::process::{Command, Stdio};

/// Builds the binary crate we've prepared solely for the purposes of testing
/// this library. This goes through cargo, so it should function identically to
/// `cargo build --bin testbin`.
pub fn build_mock_binary(name: &str) -> Result<OsString, TestBinaryError> {
    build_mock_binary_with_opts(name, None, [])
}

/// Same as [`build_mock_binary()`] but accepts additional arguments to specify
/// the build profile and features. To leave the profile as the default pass
/// `None`.
pub fn build_mock_binary_with_opts<'a, T>(
    name: &str,
    profile: Option<&str>,
    features: T,
) -> Result<OsString, TestBinaryError>
where
    T: IntoIterator<Item = &'a str>,
{
    let cargo_path = env!("CARGO");

    let mut cargo_args = vec!["build", "--message-format=json", "-q", "--bin", name];

    if let Some(prof) = profile {
        cargo_args.push("--profile");
        cargo_args.push(prof);
    }

    for feature in features {
        cargo_args.push("--features");
        cargo_args.push(feature);
    }

    let mut cargo_command = Command::new(cargo_path)
        .args(cargo_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let reader = BufReader::new(
        // The child process' stdout being None is legitimately a programming
        // error, since we created it ourselves two lines ago.
        cargo_command
            .stdout
            .take()
            .expect("cargo subprocess output has already been claimed"),
    );
    let mut messages = Message::parse_stream(reader);
    let binary_path = messages
        .find_map(|m| match m {
            Ok(Message::CompilerArtifact(artf)) if (artf.target.name == name) => Some(artf),
            _ => None,
        })
        .and_then(|a| a.executable)
        .ok_or_else(|| TestBinaryError::BinaryNotFound(name.to_owned()));

    // See above re. stderr being None.
    let mut error_reader = BufReader::new(
        cargo_command
            .stderr
            .take()
            .expect("cargo subprocess error output has already been claimed"),
    );

    let mut error_msg = String::new();

    error_reader.read_to_string(&mut error_msg)?;

    if cargo_command.wait()?.success() {
        Ok(binary_path?.into())
    } else {
        Err(TestBinaryError::CargoFailure(error_msg))
    }
}

/// Error type for build result.
#[derive(thiserror::Error, Debug)]
pub enum TestBinaryError {
    #[error("error running cargo")]
    CargoRunError(#[from] std::io::Error),
    #[error("cargo failed (message is stderr only)")]
    CargoFailure(String),
    #[error(r#"could not find binary "{0}" in cargo output"#)]
    BinaryNotFound(String),
}
