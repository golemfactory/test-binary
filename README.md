[![Maintenance](https://img.shields.io/badge/maintenance-active-success?style=for-the-badge)]()
[![License](https://img.shields.io/badge/license-MIT-informational?style=for-the-badge)](./LICENSE.md)
[![Coverage](https://img.shields.io/gitlab/coverage/detly/test-binary/main?style=for-the-badge)]()
[![Rust](https://img.shields.io/badge/rust-^1.57-informational?style=for-the-badge)]()

# test-binary

<!-- cargo-rdme start -->

Test binary generation for integration tests under Cargo.

If you have integration tests for things that involve subprocess management,
inter-process communication, or platform tools, you might need to write some
mock binaries of your own to test against. For example, if you want to test
that your code does the right thing when processing exit status codes for a
subprocess it manages, you might want a test binary that simply exits with a
certain status code. If you're testing your side of an IPC exchange, you'll
need a binary that sends the right replies.

And if you're already using Cargo to build and test, it would be nice to be
able to write those test binaries in Rust, near to the crate you're testing,
as cargo projects themselves. Then at least you'll know that your test
environments will already have the right toolchain installed.

This crate provides a simple interface for invoking Cargo to build test
binaries organised in a separate directory under your crate. *To some extent
this is already possible without using this crate at all!* If you want a
mock binary, you could put it under your `src/bin` or `examples` directory
and use it that way.

But there are reasons to use this crate instead:

- Crate binaries eg. under `src/bin`, or listed under `[[bin]]` in
  `Cargo.toml`, can be found when running tests via the environment variable
  [`CARGO_BIN_EXE_<name>`][cargo-env]. But they have to share dependencies
  with your entire crate! So whatever your mock binaries depend on, your
  entire crate has to depend on as well. This is discussed in [Cargo issue
  #1982][cargo-1982]

    [cargo-env]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    [cargo-1982]: https://github.com/rust-lang/cargo/issues/1982

- Example binaries (under `examples/` or `[[example]]`) use
  `[dev-dependencies]` instead. But they have no equivalent environment
  variable, and might not be built by the time your test runs.

- More philosophically, mock binaries are not examples. They might not use
  any aspect of your crate whatsoever. They might deliberately malfunction.
  It might be confusing to end users to find these alongside your other
  examples. It might just not be the kind of organisation you want for your
  tests.

This crate provides a way to work around those limitations.

The first thing to note is that these test binaries *aren't* binaries listed
in your actual project's manifest. So start by picking a directory name and
put them in there eg. this project uses `testbins`. **This is not going to
be a workspace.**

Under this directory you will have each test binary as a separate Cargo
binary project.

The structure should look something like this:

```none
├── Cargo.toml        (this crate's manifest)
├── src
│  └── lib.rs         (this crate's lib.rs)
├── testbins          (all the test binary projects are under this
│  │                   directory)
│  ├── test-something (one test binary)
│  │  ├── Cargo.toml  (test binary manifest, name = "test-something")
│  │  └── src
│  │     └── main.rs  (test binary source)
│  ├── test-whatever  (another test binary)
│  │  ├── Cargo.toml
│  │  └── src
│  │     └── main.rs
│   ...etc...
└── tests
   └── tests.rs       (tests for this crate, which want to use the test
                       binaries above)
```

> ### Note
>
> It can be useful to put an empty `[workspace]` section in the `Cargo.toml`
> for these test binaries, so that Cargo knows not to [look in parent
> directories][cargo-10872].

  [cargo-10872]: https://github.com/rust-lang/cargo/issues/10872

With this setup, you can now call [`build_test_binary("test-something",
"testbins")`](https://docs.rs/test-binary/latest/test_binary/fn.build_test_binary.html) where:

- `"test-something"` is the binary name you'd pass to Cargo *in the child
  project* eg. `cargo build --bin test-something`; it also has to be the
  name of the subdirectory this project is in
- `"testbins"` is the directory relative to your real project's manifest
  containing this test binary project (and maybe others)

If you need to change profiles or features, or have more control over the
directory structure, there is also [a builder API](https://docs.rs/test-binary/latest/test_binary/struct.TestBinary.html). Also
see [`build_test_binary_once!()`](https://docs.rs/test-binary/latest/test_binary/macro.build_test_binary_once.html) for a macro
that lazily builds the binary and caches the path.

Here's an example of how you might use this in a test, with a binary named
`does-build`:

```rust
let test_bin_path = build_test_binary("does-build", "testbins")
    .expect("error building test binary");
let mut test_bin_subproc = std::process::Command::new(test_bin_path)
    .spawn()
    .expect("Error running test binary");

// Test behaviour of your program against the mock binary eg. send it
// something on stdin and assert what it prints on stdout, do some IPC,
// check for side effects.

assert!(test_bin_subproc
    .wait()
    .expect("error waiting for test binary")
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
