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
            Message::CompilerArtifact(artf) if (artf.target.name == binary_name) => {
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
