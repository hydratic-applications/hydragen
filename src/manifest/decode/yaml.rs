//! Compatibility guard for serde-saphyr 2.0's overflowing-integer string fallback.
//!
//! YAML syntax, scalar decoding, styles, and resolved explicit tags come from the
//! maintained parser. Only the core schema's integer spelling/range is checked here.

use serde_saphyr::Budget;
use serde_saphyr::granit_parser::{self as parser, Event, ScalarStyle};

pub(super) fn check_integer_ranges(text: &str) -> Result<(), String> {
    inspect_integer_events(text, &Budget::default())
}

fn inspect_integer_events(text: &str, budget: &Budget) -> Result<(), String> {
    // Called only after normal deserialization has enforced all parser/alias
    // budgets. This second pass is streaming and does not expand aliases: each
    // original anchor definition is checked once, with its own style and tag.
    // Bound the pass itself too, rather than relying solely on that precondition.
    let options = parser::options! {
        emit_comments: false,
        simple_key_max_lookahead: budget.simple_key_max_lookahead,
        flow_nesting_limit: budget.flow_nesting_limit,
    };
    let mut depth = 0;
    let mut scalar_bytes = 0usize;
    for (index, item) in parser::Parser::new_from_str_with_options(text, options).enumerate() {
        if index >= budget.max_events {
            return Err("YAML integer inspection exceeded the parser event budget.".into());
        }
        let (event, span) = item.map_err(|error| error.to_string())?;
        match event {
            Event::MappingStart(..) | Event::SequenceStart(..) => {
                depth += 1;
                if depth > budget.max_depth {
                    return Err("YAML integer inspection exceeded the parser depth budget.".into());
                }
            }
            Event::MappingEnd | Event::SequenceEnd => depth -= 1,
            Event::Scalar(value, style, _, tag) => {
                scalar_bytes = scalar_bytes.saturating_add(value.len());
                if scalar_bytes > budget.max_total_scalar_bytes {
                    return Err(
                        "YAML integer inspection exceeded the parser scalar-byte budget.".into(),
                    );
                }
                let integer_candidate = match tag.as_deref() {
                    // All other explicit tags, especially !!str and !!float,
                    // retain their normal semantics, independently of style.
                    Some(tag) => tag.is_yaml_core_schema_tag("int"),
                    None => style == ScalarStyle::Plain,
                };
                if integer_candidate && core_integer_overflows(&value) {
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

fn core_integer_overflows(value: &str) -> bool {
    // YAML 1.2.2 §10.3.2: [-+]?[0-9]+ | 0o[0-7]+ | 0x[0-9a-fA-F]+.
    // https://yaml.org/spec/1.2.2/#1032-tag-resolution
    // Signs apply to decimal only. No underscores, binary prefix, or other
    // numeric extensions are introduced by this narrowly scoped guard.
    let (digits, radix, negative) = if let Some(digits) = value.strip_prefix("0x") {
        (digits, 16, false)
    } else if let Some(digits) = value.strip_prefix("0o") {
        (digits, 8, false)
    } else if let Some(digits) = value.strip_prefix('-') {
        (digits, 10, true)
    } else {
        (value.strip_prefix('+').unwrap_or(value), 10, false)
    };
    // Classify the ENTIRE spelling before checking range. An arbitrarily long
    // numeric-looking string with a non-digit suffix is still an ordinary string.
    if digits.is_empty() || !digits.bytes().all(|byte| char::from(byte).is_digit(radix)) {
        return false;
    }
    let limit = if negative {
        i64::MIN.unsigned_abs()
    } else {
        u64::MAX
    };
    digits
        .bytes()
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
                inspect_integer_events(text, &budget.unwrap())
                    .unwrap_err()
                    .contains(diagnostic)
            );
        }
    }

    #[test]
    fn guard_pass_does_not_expand_aliases() {
        let text = "name: x\na: &a [0, 0]\nb: &b [*a, *a]\nc: &c [*b, *b]\nd: [*c, *c]";
        assert!(
            inspect_integer_events(text, &serde_saphyr::budget! { max_events: 40 }.unwrap())
                .is_ok()
        );
    }
}
