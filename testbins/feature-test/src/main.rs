#[cfg(any(feature = "broken", not(feature = "working")))]
compile_error!(r#"This crate will only build if the "working" feature is enabled"#);

#[cfg(feature = "working")]
fn main() {}
