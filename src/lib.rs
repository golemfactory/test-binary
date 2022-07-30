//! Test binary generation for integration tests under Cargo.
//!
//! > ## Note
//! >
//! > This crate primarily exists to work around [Cargo issue #1982][cargo-1982]. If
//! > that has been fixed, you probably don't need this.
//!
//!   [cargo-1982]: https://github.com/rust-lang/cargo/issues/1982
//!
//! If you have integration tests for things that involve subprocess management,
//! inter-process communication, or platform tools, you might need to write some
//! mock binaries of your own to test against. And if you're already using Cargo
//! to build and test, it would be nice to be able to write those test binaries
//! in Rust, near to the crate you're testing, as cargo projects themselves.
//!
//! This crate provides a simple interface for invoking Cargo to build test
//! binaries organised in a separate directory under your crate.
//!
//! The first thing to note is that these test binaries *aren't* binaries listed
//! in your actual project's manifest. If that's what you have and it works, you
//! don't need this crate at all — you can just use
//! [`CARGO_BIN_EXE_<name>`][cargo-env].
//!
//!   [cargo-env]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
//!
//! But maybe the test binaries have to be made into separate projects because
//! they have extra dependencies. Pick a directory name and put them in there
//! eg. this project uses `testbins`. *This is not going to be a workspace.*
//! Under this directory you will have each test binary as a separate Cargo
//! project, just like any other Rust binary.
//!
//! The structure should look something like this:
//!
//! ```none
//! ├── Cargo.toml        (this crate's manifest)
//! ├── src
//! │  └── lib.rs         (this crate's lib.rs)
//! ├── testbins          (all the test binary projects are under this
//! │  │                   directory)
//! │  ├── test-something (one test binary)
//! │  │  ├── Cargo.toml  (test binary manifest, name = "test-something")
//! │  │  └── src
//! │  │     └── main.rs  (test binary source)
//! │  ├── test-whatever  (another test binary)
//! │  │  ├── Cargo.toml
//! │  │  └── src
//! │  │     └── main.rs
//! │   ...etc...
//! └── tests
//!    └── tests.rs       (tests for this crate, which want to use the test
//!                        binaries above)
//! ```
//!
//! > ## Note
//! >
//! > It can be useful to put an empty `[workspace]` section in the `Cargo.toml`
//! > for these test binaries, so that Cargo knows not to [look in parent
//! > directories][cargo-10872].
//!
//!   [cargo-10872]: https://github.com/rust-lang/cargo/issues/10872
//!
//! With this setup, you can now call [`build_test_binary("test-something",
//! "testdir")`](build_test_binary) where:
//!
//! - `"test-something"` is the binary name you'd pass to Cargo in the child
//!   project eg. `cargo build --bin test-something`; it also has to be the name
//!   of the subdirectory this project is in
//! - `"testdir"` is the directory relative to your real project's manifest
//!   containing this test binary project (and maybe others)
//!
//! If you need to change profiles or features, or have more control over the
//! directory structure, there is also [a builder API](TestBinary).
//!
//! Here's an example of how you might use this in a test, with a binary named
//! `does-build`:
//!
//! ```rust
//! # use test_binary::build_test_binary;
//! let test_bin_path = build_test_binary("does-build", "testbins")
//!     .expect("error building test binary");
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
//!     .expect("error waiting for test binary")
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

#![warn(missing_docs, missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use cargo_metadata::{BuildFinished, Message};
use std::ffi::OsString;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// For the build_mock_binary_once macro.
pub use once_cell;
pub use paste;

macro_rules! args {
    ($($item:expr),* $(,)?) => {
        vec![
            $(::std::ffi::OsString::from($item),)+
        ]
    };
}

macro_rules! add_arg {
    ($args:expr, $item:expr) => {
        $args.push(::std::ffi::OsString::from($item))
    };
}

pub struct TestBinary<'a> {
    binary: &'a str,
    manifest: &'a Path,
    features: Vec<&'a str>,
    profile: Option<&'a str>,
}

impl<'a> TestBinary<'a> {
    pub fn relative_to_parent(name: &'a str, manifest: &'a Path) -> Self {
        Self {
            binary: name,
            manifest,
            features: vec![],
            profile: None,
        }
    }

    pub fn with_profile(&mut self, profile: &'a str) -> &mut Self {
        self.profile = Some(profile);
        self
    }

    pub fn with_feature(&mut self, feature: &'a str) -> &mut Self {
        self.features.push(feature);
        self
    }

    /// Builds the binary crate we've prepared solely for the purposes of
    /// testing this library. This goes through cargo, so it should function
    /// identically to `cargo build --bin testbin`.
    pub fn build(&mut self) -> Result<OsString, TestBinaryError> {
        fn get_cargo_env_or_panic(key: &str) -> OsString {
            std::env::var_os(key).unwrap_or_else(|| {
                panic!(
                    "{} '{}' {}",
                    "The environment variable ",
                    key,
                    "is not set, is this running under a 'cargo test' command?"
                )
            })
        }

        let cargo_path = get_cargo_env_or_panic("CARGO");

        // Resolve test binary project manifest.
        let mut manifest_path = PathBuf::from(get_cargo_env_or_panic("CARGO_MANIFEST_DIR"));
        manifest_path.push(self.manifest);

        let mut cargo_args = args![
            "build",
            "--message-format=json",
            "-q",
            "--manifest-path",
            manifest_path,
            "--bin",
            self.binary,
        ];

        if let Some(prof) = self.profile {
            add_arg!(cargo_args, "--profile");
            add_arg!(cargo_args, prof);
        }

        for feature in &self.features {
            add_arg!(cargo_args, "--features");
            add_arg!(cargo_args, feature);
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
        let cargo_outcome = messages.find_map(|m| match m {
            Ok(Message::CompilerArtifact(artf)) if (artf.target.name == self.binary) => Some(
                artf.executable
                    .ok_or_else(|| TestBinaryError::BinaryNotBuilt(self.binary.to_owned())),
            ),
            Ok(Message::BuildFinished(BuildFinished { success: false, .. })) => {
                Some(Err(TestBinaryError::BuildError))
            }
            _ => None,
        });

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
            // The process succeeded. There should be a result from the JSON
            // output above.
            cargo_outcome
                .expect("Cargo succeeded but produced no output")
                .map(Into::into)
        } else if let Some(Err(err)) = cargo_outcome {
            // The process failed and there's an error we extracted from the
            // JSON output. Usually this means a compiler error.
            Err(err)
        } else {
            // The process failed but there's no error from the JSON output.
            // This will happen if there's an invocation error eg. the manifest
            // does not exist.
            //
            // This case also covers process failure but an Ok() result from the
            // above message parsing. This would be strange (if it's even
            // possible), but if it happens we should still report the error.
            Err(TestBinaryError::CargoFailure(error_msg))
        }
    }
}

pub fn build_test_binary<R: AsRef<Path>>(
    name: &str,
    directory: R,
) -> Result<OsString, TestBinaryError> {
    TestBinary::relative_to_parent(
        name,
        &PathBuf::from_iter([directory.as_ref(), name.as_ref(), "Cargo.toml".as_ref()]),
    )
    .build()
}

/// Error type for build result.
#[derive(thiserror::Error, Debug)]
pub enum TestBinaryError {
    /// An error running cargo itself.
    #[error("IO error running Cargo")]
    CargoRunError(#[from] std::io::Error),
    /// Cargo ran but did not succeed.
    #[error("Cargo failed, stderr: {0}")]
    CargoFailure(String),
    /// Cargo ran but there was a compilation error.
    #[error("build error")]
    BuildError,
    /// Cargo ran and seemed to succeed but the requested binary did not appear
    /// in its build output.
    #[error(r#"could not find binary "{0}" in Cargo output"#)]
    BinaryNotBuilt(String),
}

/// Generate a singleton function to save invoking cargo multiple times for the
/// same binary.
///
/// This is useful when you have many integration tests that use the one test
/// binary, and don't want to invoke Cargo over and over for each one. Note that
/// Cargo itself implements both locking and caching at the filesystem level, so
/// all this macro will save you is the overhead of spawning the Cargo process
/// to do its checks. That may still be appreciable for high numbers of tests or
/// on slow systems.
///
/// Calling `build_test_binary_once!(binary_name, "tests_dir")` (no quotes on
/// `binary_name`) will generate a function `path_to_binary_name()` that returns
/// the path of the built test binary as an `OsString`, just like
/// `build_test_binary("binary_name", "tests_dir")` would. Unlike
/// `build_test_binary()`, the generated function will only build the binary
/// once, and only on the first call. Subsequent calls will use a cached path
/// and assume the initial build is still valid. The generated function unwraps
/// the result internally and will panic on build errors.
///
/// For example, if you use `build_test_binary_once!(my_test, "testbins")` in
/// `tests/common/mod.rs`, that module will then contain a function
/// `path_to_my_test() -> std::ffi::OsString`. Multiple integration tests can
/// then use `common::path_to_my_test()` to obtain the path. Cargo will only be
/// run once for this binary, even if the integration tests that use it are
/// being run in multiple threads.
///
/// See this module's own integration tests for an example. If you need to use
/// extra features or a non-default profile, you will need to go back to using
/// the builder.
#[macro_export]
macro_rules! build_test_binary_once {
    ($name:ident, $tests_dir:expr) => {
        $crate::paste::paste! {
            pub fn [<path_to_ $name>]() -> std::ffi::OsString {
                use $crate::once_cell::sync::Lazy;
                use std::ffi::OsString;

                static [<lazy_path_to_ $name>]: Lazy<OsString> =
                    Lazy::new(|| $crate::build_test_binary(
                        stringify!($name),
                        $tests_dir
                    ).unwrap());
                [<lazy_path_to_ $name>].clone()
            }
        }
    };
}
