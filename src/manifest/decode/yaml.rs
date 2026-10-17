//! Bounded checks for unsupported YAML scalar data before shared-value validation.
//!
//! YAML syntax, scalar decoding, styles, and resolved explicit tags come from the
//! maintained parser. Reject binary tags and overflowing integers, including the
//! integer spellings already recognized by serde-saphyr 2.0.

use serde_saphyr::Budget;
use serde_saphyr::granit_parser::{self as parser, Event, ScalarStyle};

pub(super) fn check_scalar_data(text: &str) -> Result<(), String> {
    inspect_scalar_events(text, &Budget::default())
}

fn inspect_scalar_events(text: &str, budget: &Budget) -> Result<(), String> {
    // This pass is streaming and does not expand aliases: each original anchor
    // definition is checked once, with its own style and resolved tag. Enforce
    // this pass's own budgets; normal deserialization still enforces alias and
    // expanded-document limits independently.
    let options = parser::options! {
        emit_comments: false,
        simple_key_max_lookahead: budget.simple_key_max_lookahead,
        flow_nesting_limit: budget.flow_nesting_limit,
    };
    let mut depth = 0;
    let mut scalar_bytes = 0usize;
    for (index, item) in parser::Parser::new_from_str_with_options(text, options).enumerate() {
        if index >= budget.max_events {
            return Err("YAML scalar inspection exceeded the parser event budget.".into());
        }
        let (event, span) = item.map_err(|error| error.to_string())?;
        // Match serde-saphyr 2.0's four resolved binary identities, including
        // local/legacy compatibility tags. Display joins the parser-resolved
        // handle and suffix, not the spelling of a source token or directive.
        if event.tag().is_some_and(|tag| {
            matches!(
                tag.to_string().as_str(),
                "!!binary" | "!binary" | "tag:yaml.org,2002:binary" | "tag:yaml.org,2002:!binary"
            )
        }) {
            return Err(format!(
                "YAML binary tags are not supported in manifests at line {}, column {}; use an ordinary string or !!str for text.",
                span.start.line(),
                span.start.col() + 1,
            ));
        }
        match event {
            Event::MappingStart(..) | Event::SequenceStart(..) => {
                depth += 1;
                if depth > budget.max_depth {
                    return Err("YAML scalar inspection exceeded the parser depth budget.".into());
                }
            }
            Event::MappingEnd | Event::SequenceEnd => depth -= 1,
            Event::Scalar(value, style, _, tag) => {
                scalar_bytes = scalar_bytes.saturating_add(value.len());
                if scalar_bytes > budget.max_total_scalar_bytes {
                    return Err(
                        "YAML scalar inspection exceeded the parser scalar-byte budget.".into(),
                    );
                }
                let integer_candidate = match tag.as_deref() {
                    // All other explicit tags, especially !!str and !!float,
                    // retain their normal semantics, independently of style.
                    Some(tag) => tag.is_yaml_core_schema_tag("int"),
                    None => style == ScalarStyle::Plain,
                };
                if integer_candidate && integer_overflows(&value) {
                    return Err(format!(
                        "YAML integer is outside the supported i64/u64 range at line {}, column {}; quote it or use !!str to store text.",
                        span.start.line(),
                        span.start.col() + 1,
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn integer_overflows(value: &str) -> bool {
    // Match serde-saphyr 2.0's recognized integer extensions: an optional sign,
    // lowercase/uppercase radix prefix, and single underscores between digits.
    // The parser still determines the type; this only prevents overflow fallback.
    let value = value.trim_matches([' ', '\t', '\r', '\n']);
    let (unsigned, negative) = if let Some(unsigned) = value.strip_prefix('-') {
        (unsigned, true)
    } else {
        (value.strip_prefix('+').unwrap_or(value), false)
    };
    let (digits, radix) = if let Some(digits) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        (digits, 16)
    } else if let Some(digits) = unsigned
        .strip_prefix("0o")
        .or_else(|| unsigned.strip_prefix("0O"))
    {
        (digits, 8)
    } else if let Some(digits) = unsigned
        .strip_prefix("0b")
        .or_else(|| unsigned.strip_prefix("0B"))
    {
        (digits, 2)
    } else {
        (unsigned, 10)
    };
    // Classify the ENTIRE spelling before checking range. An arbitrarily long
    // numeric-looking string with a non-digit suffix is still an ordinary string.
    let bytes = digits.as_bytes();
    if digits.is_empty()
        || !bytes.iter().enumerate().all(|(index, &byte)| {
            if byte == b'_' {
                index > 0
                    && index + 1 < bytes.len()
                    && bytes[index - 1] != b'_'
                    && bytes[index + 1] != b'_'
            } else {
                char::from(byte).is_digit(radix)
            }
        })
    {
        return false;
    }
    // Preserve the earlier core digit-only guard. Underscored decimals with a
    // redundant leading zero are not recognized integers by this parser.
    if radix == 10 && digits.starts_with('0') && digits.contains('_') {
        return false;
    }
    let limit = if negative {
        i64::MIN.unsigned_abs()
    } else {
        u64::MAX
    };
    digits
        .bytes()
        .filter(|byte| *byte != b'_')
        .try_fold(0u64, |magnitude, byte| {
            magnitude
                .checked_mul(u64::from(radix))?
                .checked_add(u64::from(char::from(byte).to_digit(radix)?))
                .filter(|value| *value <= limit)
        })
        .is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_pass_enforces_its_own_work_limits() {
        for (budget, text, diagnostic) in [
            (
                serde_saphyr::budget! { max_events: 4 },
                "name: x\nextra: [1, 2, 3]",
                "event budget",
            ),
            (
                serde_saphyr::budget! { max_depth: 2 },
                "name: x\nextra: [[0]]",
                "depth budget",
            ),
            (
                serde_saphyr::budget! { max_total_scalar_bytes: 5 },
                "name: example",
                "scalar-byte budget",
            ),
        ] {
            assert!(
                inspect_scalar_events(text, &budget.unwrap())
                    .unwrap_err()
                    .contains(diagnostic)
            );
        }
    }

    #[test]
    fn guard_pass_does_not_expand_aliases() {
        let text = "name: x\na: &a [0, 0]\nb: &b [*a, *a]\nc: &c [*b, *b]\nd: [*c, *c]";
        assert!(
            inspect_scalar_events(text, &serde_saphyr::budget! { max_events: 40 }.unwrap()).is_ok()
        );
    }
}
