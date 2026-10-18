//! Display-only escaping for untrusted diagnostic fields, never input validation.
//!
//! Wrap raw references, paths (`path.display()`), and external parser/I/O details
//! at their first display boundary. Do not wrap composed Hydragen diagnostics:
//! their newlines and styling are intentional, and nested structured errors escape
//! their own raw fields. Backslashes and ordinary Unicode stay unchanged.
//!
//! LF, CR, and tab become `\n`, `\r`, and `\t`; other C0 controls and DEL become
//! `\xNN`. C1 controls, Unicode line/paragraph separators, and bidi formatting
//! controls become `\u{...}`. This is readable diagnostic text, not a path codec.

use std::fmt::{self, Write};

/// A borrowed-data-friendly display adapter. Formatting streams through a small
/// escaping writer without allocating a copy or accessing the filesystem.
pub struct DiagnosticText<T>(T);

pub fn display<T: fmt::Display>(value: T) -> DiagnosticText<T> {
    DiagnosticText(value)
}

impl<T: fmt::Display> fmt::Display for DiagnosticText<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(EscapingWriter(formatter), "{}", self.0)
    }
}

struct EscapingWriter<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl fmt::Write for EscapingWriter<'_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for character in text.chars() {
            match character {
                '\n' => self.0.write_str("\\n")?,
                '\r' => self.0.write_str("\\r")?,
                '\t' => self.0.write_str("\\t")?,
                // All C0 and C1 controls, including ESC, DEL and C1 CSI.
                '\u{0}'..='\u{1f}' | '\u{7f}' => write!(self.0, "\\x{:02x}", character as u32)?,
                '\u{80}'..='\u{9f}'
                // Unicode line/paragraph separators and directional formatting
                // can spoof diagnostic lines or reorder their visible meaning.
                | '\u{2028}' | '\u{2029}'
                | '\u{061c}' | '\u{200e}' | '\u{200f}'
                | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' => {
                    write!(self.0, "\\u{{{:x}}}", character as u32)?;
                }
                other => self.0.write_char(other)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_every_c0_c1_control_and_directional_or_line_formatting_character() {
        for codepoint in (0..=0x1f).chain(0x7f..=0x9f).chain([
            0x061c, 0x200e, 0x200f, 0x2028, 0x2029, 0x202a, 0x202b, 0x202c, 0x202d, 0x202e, 0x2066,
            0x2067, 0x2068, 0x2069,
        ]) {
            let character = char::from_u32(codepoint).unwrap();
            let escaped = display(character).to_string();
            assert!(!escaped.contains(character), "{codepoint:x}: {escaped:?}");
            assert!(escaped.is_ascii());
            assert!(escaped.starts_with('\\'));
        }
        assert_eq!(
            display("\0\t\n\r\u{1b}\u{7f}\u{85}").to_string(),
            r"\x00\t\n\r\x1b\x7f\u{85}"
        );
    }

    #[test]
    fn preserves_ordinary_text_and_literal_escapes_and_leaves_layout_to_the_caller() {
        let raw = r"C:\users\配置 🦀\n\x1b[2J";
        assert_eq!(display(raw).to_string(), raw);
        assert_eq!(
            format!("Heading\n  {}\n", display("a\nb")),
            "Heading\n  a\\nb\n"
        );
        // Literal escape spellings stay intact at another raw-data boundary too.
        assert_eq!(display(display("a\nb")).to_string(), r"a\nb");
    }

    #[test]
    fn escapes_each_fragment_of_an_external_display_implementation() {
        struct Fragments;
        impl fmt::Display for Fragments {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("first\u{1b}")?;
                formatter.write_str("[2J\n")?;
                formatter.write_char('\u{9b}')
            }
        }
        assert_eq!(display(Fragments).to_string(), r"first\x1b[2J\n\u{9b}");
    }
}
