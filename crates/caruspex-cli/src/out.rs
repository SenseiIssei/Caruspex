//! Output helpers: stdout and stderr writes that never panic (a closed pipe
//! is not an error worth dying for), and C-style padding by bytes.

use std::io::Write;

pub fn say(text: &str) {
    let _ = std::io::stdout().write_all(text.as_bytes());
}

pub fn say_err(text: &str) {
    let _ = std::io::stderr().write_all(text.as_bytes());
}

pub fn flush() {
    let _ = std::io::stdout().flush();
}

/// printf("%-Ns"): pads to `width` bytes, not characters, like the C library.
pub fn ljust(text: &str, width: usize) -> String {
    let mut s = text.to_string();
    while s.len() < width {
        s.push(' ');
    }
    s
}

/// printf("%Ns"): the same, padding on the left.
pub fn rjust(text: &str, width: usize) -> String {
    let mut s = String::new();
    while s.len() + text.len() < width {
        s.push(' ');
    }
    s.push_str(text);
    s
}
