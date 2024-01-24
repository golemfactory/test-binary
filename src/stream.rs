//! Stream handling and parsing code. This is the more "pure, functional" aspect
//! of the test binary code.

use crate::TestBinaryError;
use camino::Utf8PathBuf;
use cargo_metadata::Message;
use std::{fmt::Write as _, io::BufRead};

/// Process a stream of messages from Cargo's output, searching for the binary
/// name we want or gathering information for a useful error.
pub(super) fn process_messages<R: BufRead>(
    reader: R,
    binary_name: &str,
) -> Option<Result<Utf8PathBuf, TestBinaryError>> {
    // Parse messages with cargo_metadata.
    let messages = Message::parse_stream(reader);

    // The actual outcome is we either find the path and return it, or generate
    // an error.
    let mut cargo_outcome = None;

    // Keep these in case the build fails.
    let mut compiler_messages = String::new();

    for message in messages.flatten() {
        match message {
            // Hooray we found it!
            Message::CompilerArtifact(artf)
                if (artf.target.name == binary_name
                    && artf.target.kind.contains(&"bin".to_string())) =>
            {
                cargo_outcome = Some(artf.executable.ok_or_else(|| {
                    // Wait no we didn't.
                    TestBinaryError::BinaryNotBuilt(binary_name.to_owned())
                }));
                break;
            }

            // Let's keep these just in case.
            Message::CompilerMessage(msg) => {
                writeln!(compiler_messages, "{}", msg).expect("error writing to String");
            }
            Message::TextLine(text) => {
                writeln!(compiler_messages, "{}", text).expect("error writing to String");
            }

            // Hooray it's finished!
            Message::BuildFinished(build_result) => {
                cargo_outcome = if build_result.success {
                    cargo_outcome.or_else(|| {
                        // Wait our binary isn't there.
                        Some(Err(TestBinaryError::BinaryNotBuilt(binary_name.to_owned())))
                    })
                } else {
                    // Wait it failed.
                    Some(Err(TestBinaryError::BuildError(compiler_messages)))
                };
                break;
            }

            _ => continue,
        }
    }

    cargo_outcome
}

#[cfg(test)]
mod tests {
    //! The "good" path is mostly tested by integration tests. These mostly test
    //! the error handling and rendering.

    use super::*;
    use indoc::indoc;

    #[test]
    fn regular_error() {
        let binary = "fla";
        let json_output = indoc! {r##"
{"reason":"compiler-message","package_id":"fla 0.1.0 (path+file:///test-binary/testbins/fla)","manifest_path":"/test-binary/testbins/fla/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"fla","src_path":"/test-binary/testbins/fla/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"message":{"rendered":"error: unknown start of token: \\u{1f9a9}\n --> src/main.rs:1:13\n  |\n1 | fn main() { 🦩 }\n  |             ^^\n\n","children":[],"code":null,"level":"error","message":"unknown start of token: \\u{1f9a9}","spans":[{"byte_end":16,"byte_start":12,"column_end":14,"column_start":13,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":null,"line_end":1,"line_start":1,"suggested_replacement":null,"suggestion_applicability":null,"text":[{"highlight_end":14,"highlight_start":13,"text":"fn main() { 🦩 }"}]}]}}
{"reason":"compiler-message","package_id":"fla 0.1.0 (path+file:///test-binary/testbins/fla)","manifest_path":"/test-binary/testbins/fla/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"fla","src_path":"/test-binary/testbins/fla/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"message":{"rendered":"error: aborting due to previous error\n\n","children":[],"code":null,"level":"error","message":"aborting due to previous error","spans":[]}}
{"reason":"build-finished","success":false}
"##};

        let expected_msg = indoc! {r#"
error: unknown start of token: \u{1f9a9}
 --> src/main.rs:1:13
  |
1 | fn main() { 🦩 }
  |             ^^


error: aborting due to previous error


"#};

        let outcome = process_messages(std::io::Cursor::new(json_output), binary);

        if let Some(Err(TestBinaryError::BuildError(msg))) = outcome {
            assert_eq!(msg, expected_msg);
        } else {
            panic!("{:#?}", outcome);
        }
    }

    #[test]
    fn error_with_line() {
        let binary = "fla";
        let json_output = indoc! {r##"
{"reason":"compiler-message","package_id":"fla 0.1.0 (path+file:///test-binary/testbins/fla)","manifest_path":"/test-binary/testbins/fla/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"fla","src_path":"/test-binary/testbins/fla/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"message":{"rendered":"error: unknown start of token: \\u{1f9a9}\n --> src/main.rs:1:13\n  |\n1 | fn main() { 🦩 }\n  |             ^^\n\n","children":[],"code":null,"level":"error","message":"unknown start of token: \\u{1f9a9}","spans":[{"byte_end":16,"byte_start":12,"column_end":14,"column_start":13,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":null,"line_end":1,"line_start":1,"suggested_replacement":null,"suggestion_applicability":null,"text":[{"highlight_end":14,"highlight_start":13,"text":"fn main() { 🦩 }"}]}]}}
Surprise text line!
{"reason":"compiler-message","package_id":"fla 0.1.0 (path+file:///test-binary/testbins/fla)","manifest_path":"/test-binary/testbins/fla/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"fla","src_path":"/test-binary/testbins/fla/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"message":{"rendered":"error: aborting due to previous error\n\n","children":[],"code":null,"level":"error","message":"aborting due to previous error","spans":[]}}
{"reason":"build-finished","success":false}
"##};

        let expected_msg = indoc! {r#"
error: unknown start of token: \u{1f9a9}
 --> src/main.rs:1:13
  |
1 | fn main() { 🦩 }
  |             ^^


Surprise text line!
error: aborting due to previous error


"#};

        let outcome = process_messages(std::io::Cursor::new(json_output), binary);

        if let Some(Err(TestBinaryError::BuildError(msg))) = outcome {
            assert_eq!(msg, expected_msg);
        } else {
            panic!("{:#?}", outcome);
        }
    }

    #[test]
    fn build_with_no_binary() {
        let binary = "fla";
        let json_output = indoc! {r##"
{"reason":"compiler-artifact","package_id":"fla 0.1.0 (path+file:///test-binary/testbins/fla)","manifest_path":"/test-binary/testbins/fla/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"fla","src_path":"/test-binary/testbins/fla/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/test-binary/testbins/fla/target/debug/fla"],"fresh":false}
{"reason":"build-finished","success":true}
"##};

        let outcome = process_messages(std::io::Cursor::new(json_output), binary);

        if let Some(Err(TestBinaryError::BinaryNotBuilt(name))) = outcome {
            assert_eq!(name, binary);
        } else {
            panic!("{:#?}", outcome);
        }
    }

    #[test]
    fn build_finish_early() {
        let binary = "fla";
        let json_output = indoc! {r##"
{"reason":"build-finished","success":true}
"##};

        let outcome = process_messages(std::io::Cursor::new(json_output), binary);

        if let Some(Err(TestBinaryError::BinaryNotBuilt(name))) = outcome {
            assert_eq!(name, binary);
        } else {
            panic!("{:#?}", outcome);
        }
    }
}
