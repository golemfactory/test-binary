//! Manage and build extra binaries for integration tests as regular Rust
//! crates.
//!
//! If you have integration tests for things that involve subprocess management,
//! inter-process communication, or platform tools, you might need to write some
//! extra "supporting" binaries of your own to help with these tests. For
//! example, if you want to test that your code does the right thing with the
//! exit status for a managed subprocess, you might want a supporting binary
//! that can be made to exit with a certain status code. If you're testing an
//! IPC exchange, you might want to test against a binary "mock" that sends some
//! scripted replies.
//!
//! And if you're already using Cargo to build and test, it would be nice to be
//! able to write those extra binaries in Rust, near to the crate you're
//! testing, as Cargo projects themselves. Then at least you'll know that your
//! test environments will already have the right toolchain installed.
//!
//! *To some extent this is already possible without using this crate at all!*
//! If you want an extra binary, you could put it under your `src/bin` or
//! `examples` directory and use it that way. But there are limitations to
//! what's currently possible under Cargo alone:
//!
//! - Crate binaries eg. under `src/bin`, or listed under `[[bin]]` in
//!   `Cargo.toml`, can be found via the environment variable
//!   [`CARGO_BIN_EXE_<name>`][cargo-env] when running tests. But they have to
//!   share dependencies with your entire crate! So whatever your supporting
//!   binaries depend on, your entire crate has to depend on as well. This is
//!   discussed in [Cargo issue #1982][cargo-1982]
//!
//!     [cargo-env]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
//!     [cargo-1982]: https://github.com/rust-lang/cargo/issues/1982
//!
//! - Example binaries (under `examples/` or `[[example]]`) use
//!   `[dev-dependencies]` instead. But they have no equivalent environment
//!   variable, and might not be built by the time your test runs.
//!
//! - More philosophically: such binaries are not examples, nor are they real
//!   applications. They might not use any aspect of your crate whatsoever. They
//!   might deliberately malfunction. It might be confusing to end users to find
//!   these alongside your other examples. It might just not be the kind of
//!   organisation you want for your tests.
//!
//! - Organising supporting binaries as workspace crates requires publishing
//!   every one of those crates to [`crates.io`](https://crates.io) (or whatever
//!   registry you're using), even if they have no use whatsoever outside of
//!   your crate's integration tests.
//!
//! This crate provides a way to work around those constraints. It has a simple
//! interface for invoking Cargo to build extra binaries organised in a separate
//! directory under your crate.
//!
//! The first thing to note is that these extra binaries *aren't* binaries
//! listed in your actual project's manifest. So start by picking a directory
//! name and put them in there eg. this project uses `testbins`. **This is not
//! going to be a workspace.** Under this directory you will have these extra
//! binaries in their own Cargo packages.
//!
//! The structure should look something like this:
//!
//! ```none
//! ├── Cargo.toml        (your crate's manifest)
//! ├── src
//! │  └── lib.rs         (your crate's lib.rs)
//! ├── tests
//! │  └── tests.rs       (your crate's tests, which want to use the supporting
//! │                      binaries below)
//! │
//! └── testbins          (all the extra binary projects are under this
//!    │                   directory)
//!    ├── test-something (first extra binary)
//!    │  ├── Cargo.toml  (extra binary manifest, name = "test-something")
//!    │  └── src
//!    │     └── main.rs  (extra binary source)
//!    ├── test-whatever  (another extra binary, name = "test-whatever")
//!    │  ├── Cargo.toml
//!    │  └── src
//!    │     └── main.rs
//!     ...etc...
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
//! "testbins")`](crate::build_test_binary). See how:
//!
//! - `"test-something"` is the binary name you'd pass to Cargo *in the child
//!   project* eg. if you changed directory to the nested project, you'd run
//!   `cargo build --bin test-something`; it also has to be the name of the
//!   subdirectory this project is in
//! - `"testbins"` is the directory relative to your real project's manifest
//!   containing this test binary project (and maybe others); think of it like
//!   you'd think of the `examples` or `tests` directory
//!
//! If you need to set different profiles or features, or have more control over
//! the directory structure, there is also [a builder API](crate::TestBinary).
//! Also see [`build_test_binary_once!()`](crate::build_test_binary_once) for a
//! macro that lazily builds the binary and caches the path.
//!
//! Here's an example of how you might use this crate's API in a test, with a
//! binary named `does-build`:
//!
//! ```rust
//! # use test_binary::build_test_binary;
//!
//! let test_bin_path = build_test_binary("does-build", "testbins")
//!     .expect("error building test binary");
//!
//! let mut test_bin_subproc = std::process::Command::new(test_bin_path)
//!     .spawn()
//!     .expect("error running test binary");
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
//! path is not resolved to an absolute path by this crate, although it might be
//! one anyway. Since it is the path provided by Cargo after being invoked in
//! the current process' working directory, it will be valid as long as you do
//! not change the working directory between obtaining it and using it.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
    ffi::OsString,
    io::{BufReader, Read},
    ops::Index,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

// For the build_test_binary_once macro.
pub use once_cell;
pub use paste;

mod stream;

// Internal macros for OsString boilerplate.

macro_rules! vec_oss {
    ($($item:expr),* $(,)?) => {
        vec![
            $(::std::ffi::OsString::from($item),)+
        ]
    };
}

macro_rules! push_oss {
    ($args:expr, $item:expr) => {
        $args.push(::std::ffi::OsString::from($item))
    };
}

/// Builder constructor for a test binary.
///
/// Start with [`TestBinary::relative_to_parent(name,
/// manifest)`](TestBinary::relative_to_parent) where
/// - `name` is the name of the binary in the child project's manifest
/// - `manifest` is the path to the manifest file for the test binary, relative
///   to the directory that the containing project is in. It should probably end
///   in `Cargo.toml`.
///
/// Note that you can pass a path in a cross-platform way by using
/// [`PathBuf::from_iter()`][std::path::PathBuf::from_iter()]:
///
/// ```
/// # use std::path::PathBuf;
/// # use test_binary::TestBinary;
/// TestBinary::relative_to_parent(
///     "does-build",
///     &PathBuf::from_iter(["testbins", "does-build", "Cargo.toml"]),
/// );
/// ```
#[derive(Debug)]
pub struct TestBinary<'a> {
    binary: &'a str,
    manifest: PathBuf,
    features: Vec<&'a str>,
    default_features: bool,
    profile: Option<&'a str>,
}

impl<'a> TestBinary<'a> {
    /// Creates a new `TestBinary` by specifying the child binary's manifest
    /// relative to the parent.
    pub fn relative_to_parent(name: &'a str, manifest: &'a Path) -> Result<Self, TestBinaryError> {
        let manifest_path = manifest_dir()?.join(manifest);

        Ok(Self {
            binary: name,
            manifest: manifest_path,
            features: vec![],
            default_features: true,
            profile: None,
        })
    }

    /// Find binary in workspace and create `TestBinary` struct.
    pub fn from_workspace(name: &'a str) -> Result<Self, TestBinaryError> {
        let manifest_path = find_package(name)?;
        Ok(Self {
            binary: name,
            manifest: manifest_path,
            features: vec![],
            default_features: true,
            profile: None,
        })
    }

    /// Specifies a profile to build the test binary with.
    pub fn with_profile(&mut self, profile: &'a str) -> &mut Self {
        self.profile = Some(profile);
        self
    }

    /// Specifies not to enable default features.
    pub fn no_default_features(&mut self) -> &mut Self {
        self.default_features = false;
        self
    }

    /// Specifies a feature to enable for the test binary. These are additive,
    /// so if you call this multiple times all the features you specify will be
    /// enabled.
    pub fn with_feature(&mut self, feature: &'a str) -> &mut Self {
        self.features.push(feature);
        self
    }

    /// Builds the binary crate we've prepared. This goes through Cargo, so it
    /// should function identically to `cargo build --bin testbin` along with
    /// any additional flags from the builder methods.
    pub fn build(&mut self) -> Result<OsString, TestBinaryError> {
        fn get_cargo_env(key: &str) -> Result<OsString, TestBinaryError> {
            std::env::var_os(key).ok_or_else(|| {
                TestBinaryError::NonCargoRun(format!(
                    "{} '{}' {}",
                    "The environment variable ", key, "is not set",
                ))
            })
        }

        let cargo_path = get_cargo_env("CARGO")?;
        let mut cargo_args = vec_oss![
            "build",
            "--message-format=json",
            "-q",
            "--manifest-path",
            self.manifest.clone(),
            "--bin",
            self.binary,
        ];

        if let Some(prof) = self.profile {
            push_oss!(cargo_args, "--profile");
            push_oss!(cargo_args, prof);
        }

        if !self.default_features {
            push_oss!(cargo_args, "--no-default-features");
        }

        for feature in &self.features {
            push_oss!(cargo_args, "--features");
            push_oss!(cargo_args, feature);
        }

        let mut cargo_command = Command::new(cargo_path)
            .args(cargo_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let reader = BufReader::new(
            // The child process' stdout being None is legitimately a
            // programming error, since we created it ourselves two lines ago.
            //
            // Use as_mut() instead of take() here because if we detach
            // ownership from the subprocess, we risk letting it drop
            // prematurely, which can make it close before the subprocess is
            // finished, resulting in a broken pipe error (but in a highly
            // timing/platform/performance dependent and intermittent way).
            cargo_command
                .stdout
                .as_mut()
                .expect("Cargo subprocess output has already been claimed"),
        );

        let cargo_outcome = stream::process_messages(reader, self.binary);

        // See above re. stderr being None.
        let mut error_reader = BufReader::new(
            cargo_command
                .stderr
                .as_mut()
                .expect("Cargo subprocess error output has already been claimed"),
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

/// Simplified function for building a test binary where the binary is in a
/// subdirectory of the same name, the manifest is named `Cargo.toml`, and you
/// don't need any non-default features or to specify a profile.
///
/// For example, if your parent contains the child binary in
/// `testbins/does-build`, and the binary is named `does-build` in its
/// `Cargo.toml`, then you can just call `build_test_binary("does_build",
/// "testbins")`.
pub fn build_test_binary<R: AsRef<Path>>(
    name: &str,
    directory: R,
) -> Result<OsString, TestBinaryError> {
    TestBinary::relative_to_parent(
        name,
        &PathBuf::from_iter([directory.as_ref(), name.as_ref(), "Cargo.toml".as_ref()]),
    )?
    .build()
}

fn manifest_dir() -> Result<PathBuf, ManifestError> {
    PathBuf::from_str(
        &std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| ManifestError::EnvNotSet(e.to_string()))?,
    )
    .map_err(|e| ManifestError::EnvNotSet(e.to_string()))
}

/// Locates package in current workspace.
/// Returns path to Cargo.toml defining package that will produce desired binary.
fn find_package(bin: &str) -> Result<PathBuf, ManifestError> {
    let manifest_dir = manifest_dir()?;
    let manifest_path = manifest_dir.join("Cargo.toml");
    let manifest = cargo_metadata::MetadataCommand::new()
        .manifest_path(&manifest_path)
        .exec()
        .map_err(|e| ManifestError::ReadManifest(manifest_dir.to_path_buf(), e.to_string()))?;

    let workspace_manifest = manifest.workspace_root.join("Cargo.toml");
    let workspace = cargo_metadata::MetadataCommand::new()
        .manifest_path(&workspace_manifest)
        .exec()
        .map_err(|e| {
            ManifestError::ReadManifest(workspace_manifest.into_std_path_buf(), e.to_string())
        })?;

    for id in &workspace.workspace_members {
        let package = workspace.index(id);
        if package.name == bin {
            return Ok(package.manifest_path.clone().into_std_path_buf());
        }
    }
    Err(ManifestError::PackageNotFound(bin.to_string()))
}

/// Error type for build result.
#[derive(thiserror::Error, Debug)]
pub enum TestBinaryError {
    /// We are not running under Cargo.
    #[error("{0}; is this running under a 'cargo test' command?")]
    NonCargoRun(String),
    /// An error running Cargo itself.
    #[error("IO error running Cargo")]
    CargoRunError(#[from] std::io::Error),
    /// Cargo ran but did not succeed.
    #[error("Cargo failed, stderr: {0}")]
    CargoFailure(String),
    /// Cargo ran but there was a compilation error.
    #[error("build error:\n{0}")]
    BuildError(String),
    /// Cargo ran and seemed to succeed but the requested binary did not appear
    /// in its build output.
    #[error(r#"could not find binary "{0}" in Cargo output"#)]
    BinaryNotBuilt(String),
    /// Error processing manifests.
    #[error("manifest error: {0}")]
    ManifestError(#[from] ManifestError),
}

/// Error during reading manifests.
#[derive(thiserror::Error, Debug)]
pub enum ManifestError {
    /// Workspace manifest doesn't contain info about package.
    #[error("Package {0} not found")]
    PackageNotFound(String),
    /// Error when reading manifest.
    #[error("Error reading manifest: {}. {1}", .0.display())]
    ReadManifest(PathBuf, String),
    /// Can't query path to manifest of current crate.
    #[error("ENV variable `CARGO_MANIFEST_DIR` is not set. Error: {0}")]
    EnvNotSet(String),
}

/// Generate a singleton function to save invoking Cargo multiple times for the
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
/// > ## Note
/// >
/// > That this means the binary name must be a valid identifier eg. not have
/// > dashes in it.
///
/// ```rust
/// # use test_binary::build_test_binary_once;
/// // Build a test binary named "multiple".
/// build_test_binary_once!(multiple, "testbins");
///
/// // The first test that gets run will cause the binary "multiple" to be built
/// // and the path will be cached inside the `path_to_multiple()` function.
///
/// let test_bin_path = path_to_multiple();
/// assert!(std::process::Command::new(test_bin_path)
///     .status()
///     .expect("Error running test binary")
///     .success());
///
/// // Subsequent tests will just get the cached path without spawning Cargo
/// // again.
///
/// let test_bin_path_again = path_to_multiple();
/// assert!(std::process::Command::new(test_bin_path_again)
///     .status()
///     .expect("Error running test binary")
///     .success());
/// ```
///
/// If you need to use extra features or a non-default profile, you will need to
/// go back to using the builder.
#[macro_export]
macro_rules! build_test_binary_once {
    ($name:ident, $tests_dir:expr) => {
        $crate::paste::paste! {
            pub fn [<path_to_ $name>]() -> std::ffi::OsString {
                use $crate::once_cell::sync::Lazy;
                use std::ffi::OsString;

                static [<LAZY_PATH_TO_ $name>]: Lazy<OsString> =
                    Lazy::new(|| $crate::build_test_binary(
                        stringify!($name),
                        $tests_dir
                    ).unwrap());
                [<LAZY_PATH_TO_ $name>].clone()
            }
        }
    };
}
