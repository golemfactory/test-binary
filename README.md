[![Maintenance](https://img.shields.io/badge/maintenance-active-success?style=for-the-badge)]()
[![License](https://img.shields.io/badge/license-MIT-informational?style=for-the-badge)](./LICENSE.md)
[![Coverage](https://img.shields.io/gitlab/coverage/detly/test-binary/main?style=for-the-badge)]()
[![Rust](https://img.shields.io/badge/rust-^1.57-informational?style=for-the-badge)]()

# test-binary

<!-- cargo-rdme start -->

Test binary generation for integration tests under Cargo.

If you have integration tests for things that involve subprocess management,
inter-process communication, or platform tools, you might need to write some
mock binaries of your own to test against. And if you're already using Cargo
to build and test, it would be nice to be able to write those test binaries
in Rust, in the crate you're testing, as crate binaries themselves.

This crate provides a simple interface for invoking Cargo to build test
binaries in your own crate, defined in your `Cargo.toml`. Call
[`build_mock_binary("name_of_binary")`] where `"name_of_binary"` is the
binary name you'd pass to Cargo eg. `cargo build --bin name_of_binary`. If
you need to change profiles or features, there is
[`build_mock_binary_with_opts()`].

Here's an example of how you might use this in a test, with a binary named
`test_it_builds`

```rust
let test_bin_path = build_mock_binary("test_it_builds").expect("Error building test binary");
let mut test_bin_subproc = std::process::Command::new(test_bin_path)
    .spawn()
    .expect("Error running test binary");

// Test behaviour of your program against the mock binary eg. send it
// something on stdin and assert what it prints on stdout, do some IPC,
// check for side effects.

assert!(test_bin_subproc
    .wait()
    .expect("Error waiting for test binary")
    .success());
```

The result returned by these functions contains the path of the built binary
as a [`std::ffi::OsString`], which can be passed to
[`std::process::Command`] or other crates that deal with subprocesses. The
path is not resolved to an absolute path, although it might be one anyway.
Since it is the path provided by Cargo after being invoked in the current
process' working directory, it will be valid as long as you do not change
the working directory between obtaining it and using it.

<!-- cargo-rdme end -->

---

Minimum supported Rust version: 1.57. Licensed under the MIT license (see `LICENSE` file in this directory).

This document is kept up-to-date with [cargo-rdme](https://github.com/orium/cargo-rdme).
