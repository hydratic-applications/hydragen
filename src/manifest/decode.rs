//! Parser-specific syntax policy, followed by loss-checking conversion to shared values.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use super::ManifestFormat;

mod yaml;

pub(super) fn document(text: &str, format: ManifestFormat) -> Result<Value, String> {
    match format {
        ManifestFormat::Json => serde_json::from_str::<CheckedValue>(text)
            .map(|value| value.0)
            .map_err(|error| error.to_string()),
        ManifestFormat::Jsonc => jsonc(text),
        ManifestFormat::Yaml | ManifestFormat::Yml => {
            let options = serde_saphyr::options! {
                strict_booleans: true,
                duplicate_keys: serde_saphyr::DuplicateKeyPolicy::Error,
                merge_keys: serde_saphyr::MergeKeyPolicy::Error,
                reject_unsupported_tags: true,
                non_finite_float_policy: serde_saphyr::NonFiniteFloatPolicy::Reject,
            };
            // Default parser/alias budgets remain enabled. String input is already UTF-8.
            let CheckedValue(value) =
                serde_saphyr::from_str_with_options::<CheckedValue>(text, options)
                    .map_err(|error| error.to_string())?;
            yaml::check_integer_ranges(text)?;
            Ok(value)
        }
        ManifestFormat::Toml => {
            let table: toml::Table = toml::from_str(text).map_err(|error| error.to_string())?;
            toml_value(toml::Value::Table(table))
        }
    }
}

// Both JSON and YAML use this recursive visitor, so no map can lose repeated keys.
// In particular, YAML keys must be inspected via deserialize_any, NOT String's
// type-driven deserializer (which would coerce numeric/boolean keys to strings).
struct CheckedValue(Value);
struct StringKey(String);

impl<'de> Deserialize<'de> for StringKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StringVisitor;
        impl Visitor<'_> for StringVisitor {
            type Value = StringKey;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string object key")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StringKey(value.to_owned()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(StringKey(value))
            }
        }
        deserializer.deserialize_any(StringVisitor)
    }
}

impl<'de> Deserialize<'de> for CheckedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = CheckedValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON-compatible data")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(CheckedValue(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(CheckedValue(value.into()))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(CheckedValue(value.into()))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(|number| CheckedValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("non-finite numbers are not supported"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(CheckedValue(Value::String(value.to_owned())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(CheckedValue(Value::String(value)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(CheckedValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                self.visit_unit()
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(CheckedValue(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(CheckedValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some(StringKey(key)) = object.next_key()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!("duplicate object key '{key}'")));
                    }
                    let CheckedValue(value) = object.next_value()?;
                    values.insert(key, value);
                }
                Ok(CheckedValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

fn jsonc(text: &str) -> Result<Value, String> {
    let options = jsonc_parser::ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    };
    let collect = jsonc_parser::CollectOptions {
        comments: jsonc_parser::CommentCollectionStrategy::AsTokens,
        tokens: true,
    };
    let parsed =
        jsonc_parser::parse_to_ast(text, &collect, &options).map_err(|error| error.to_string())?;
    // Even with all extensions disabled, jsonc-parser 0.34 accepts raw control
    // characters in strings and JSON5 whitespace. Validate its original string
    // tokens and inter-token gaps with serde_json, not a second tokenizer or
    // comment-stripping pass. Comments retain their own parser-provided ranges.
    let mut previous_end = 0;
    for token in parsed.tokens.as_deref().unwrap_or_default() {
        serde_json::Deserializer::from_str(&text[previous_end..token.range.start])
            .end()
            .map_err(|error| error.to_string())?;
        if matches!(token.token, jsonc_parser::tokens::Token::String(_)) {
            serde_json::from_str::<String>(&text[token.range.start..token.range.end])
                .map_err(|error| error.to_string())?;
        }
        previous_end = token.range.end;
    }
    serde_json::Deserializer::from_str(&text[previous_end..])
        .end()
        .map_err(|error| error.to_string())?;
    jsonc_value(
        parsed
            .value
            .ok_or("Expected one object/mapping document, not empty input.")?,
    )
}

fn jsonc_value(node: jsonc_parser::ast::Value<'_>) -> Result<Value, String> {
    use jsonc_parser::ast::Value as Ast;
    Ok(match node {
        Ast::StringLit(value) => Value::String(value.value.into_owned()),
        // Parse the original number with serde_json: no f64-only integer rounding,
        // no unchecked infinity, and no conversion of failed numbers into strings.
        Ast::NumberLit(value) => Value::Number(
            value
                .value
                .parse::<Number>()
                .map_err(|error| error.to_string())?,
        ),
        Ast::BooleanLit(value) => Value::Bool(value.value),
        Ast::NullKeyword(_) => Value::Null,
        Ast::Array(array) => Value::Array(
            array
                .elements
                .into_iter()
                .map(jsonc_value)
                .collect::<Result<_, _>>()?,
        ),
        Ast::Object(object) => {
            let mut values = Map::new();
            for property in object.properties {
                let key = property.name.into_string();
                if values.contains_key(&key) {
                    return Err(format!("duplicate object key '{key}'"));
                }
                values.insert(key, jsonc_value(property.value)?);
            }
            Value::Object(values)
        }
    })
}

fn toml_value(value: toml::Value) -> Result<Value, String> {
    Ok(match value {
        toml::Value::String(text) => Value::String(text),
        toml::Value::Integer(number) => number.into(),
        toml::Value::Float(number) => Value::Number(
            Number::from_f64(number)
                .ok_or("TOML non-finite numbers are not supported in manifests.")?,
        ),
        toml::Value::Boolean(value) => Value::Bool(value),
        toml::Value::Datetime(_) => return Err(
            "TOML date/time literals are not supported in manifests; use a quoted string instead."
                .into(),
        ),
        toml::Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(toml_value)
                .collect::<Result<_, _>>()?,
        ),
        toml::Value::Table(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| Ok((key, toml_value(value)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
