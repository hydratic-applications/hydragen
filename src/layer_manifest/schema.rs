//! Cached, offline validation of parsed layer manifest content using the bundled schema.

use std::fmt;
use std::sync::OnceLock;

use jsonschema::{
    Draft, Registry, Retrieve, Uri, ValidationError, Validator, error::ValidationErrorKind,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SCHEMA: &str = include_str!("../../schemas/layer-manifest.schema.json");
// An in-memory resource identifier, not a published URL or a layer manifest property.
const SCHEMA_URI: &str = "urn:hydragen:layer-manifest";
const FIELDS: [&str; 4] = ["name", "author", "description", "$schema"];
const NONBLANK_PATTERN_PATH: &str = "/$defs/nonBlankString/pattern";
const NONBLANK_APPROVAL_PATH: &str = "/$defs/nonBlankString/x-hydragen-nonblank-pattern-sha256";
static VALIDATORS: OnceLock<Result<Validators, Error>> = OnceLock::new();

/// Internal schema failures must not be presented as retryable user input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Setup {
        message: String,
        /// Structured detail when schema compilation or meta-validation failed.
        detail: Option<Box<Violation>>,
    },
    Content(Box<Violation>),
}

/// The first failed schema rule. Paths are JSON Pointers; an empty path means the root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub instance_path: String,
    pub schema_path: String,
    pub keyword: String,
    pub message: String,
    pub kind: ViolationKind,
}

/// Classifications needed for friendly metadata diagnostics, without rechecking input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViolationKind {
    NonBlank,
    Type,
    Required { property: String },
    Other,
}

impl Error {
    fn setup(message: impl fmt::Display) -> Self {
        Self::Setup {
            message: message.to_string(),
            detail: None,
        }
    }

    fn compilation(error: ValidationError<'_>) -> Self {
        Self::Setup {
            message: "The bundled layer manifest schema could not be prepared".to_owned(),
            detail: Some(Box::new(Violation::from_error(error, None, None))),
        }
    }
}

impl Violation {
    fn from_error(
        error: ValidationError<'_>,
        field: Option<&str>,
        approved_nonblank_pattern: Option<&str>,
    ) -> Self {
        let schema_path = error.schema_path().to_string();
        let kind = match error.kind() {
            // Friendly feedback requires both the named location and fingerprint
            // approval of this exact pattern, not merely whichever pattern is current.
            ValidationErrorKind::Pattern { pattern }
                if schema_path == NONBLANK_PATTERN_PATH
                    && Some(pattern.as_str()) == approved_nonblank_pattern =>
            {
                ViolationKind::NonBlank
            }
            ValidationErrorKind::Type { .. } => ViolationKind::Type,
            ValidationErrorKind::Required {
                property: Value::String(property),
            } => ViolationKind::Required {
                property: property.clone(),
            },
            _ => ViolationKind::Other,
        };
        // Field wrappers validate scalars, so prefix their paths with the known property.
        let instance_path = match field {
            Some(field) => format!("/{field}{}", error.instance_path()),
            None => error.instance_path().to_string(),
        };
        Self {
            instance_path,
            schema_path,
            keyword: error.kind().keyword().to_owned(),
            message: error.to_string(),
            kind,
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = if self.instance_path.is_empty() {
            "<root>"
        } else {
            &self.instance_path
        };
        write!(formatter, "{path} ({}): {}", self.keyword, self.message)
    }
}

impl std::error::Error for Violation {}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup { message, detail } => {
                write!(formatter, "Internal layer manifest schema error: {message}")?;
                if let Some(detail) = detail {
                    write!(formatter, ": {detail}")?;
                }
                Ok(())
            }
            Self::Content(violation) => violation.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Setup { detail, .. } => detail.as_deref().map(|detail| detail as _),
            Self::Content(violation) => Some(violation.as_ref()),
        }
    }
}

// Registry preparation has its own retriever, separate from ValidationOptions::offline.
// Reject every unavailable resource before any network or external-file operation.
struct NoRetrieval;

impl Retrieve for NoRetrieval {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(format!("External schema retrieval is disabled: {uri}").into())
    }
}

struct Validators {
    root: Validator,
    fields: Vec<(&'static str, Validator)>,
    approved_nonblank_pattern: Option<String>,
}

impl Validators {
    // A private constructor also permits focused schema variations in unit tests.
    fn compile(source: &str) -> Result<Self, Error> {
        let schema: Value = serde_json::from_str(source).map_err(Error::setup)?;
        jsonschema::draft202012::meta::validate(&schema).map_err(Error::compilation)?;
        for field in FIELDS {
            if schema.pointer(&format!("/properties/{field}")).is_none() {
                return Err(Error::setup(format!(
                    "Missing layer manifest schema definition /properties/{field}"
                )));
            }
        }
        let nonblank_pattern = schema
            .pointer(NONBLANK_PATTERN_PATH)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                Error::setup(format!(
                    "Missing layer manifest schema definition {NONBLANK_PATTERN_PATH}"
                ))
            })?;
        // This annotation approves diagnostics, not input validity. Missing, malformed,
        // or stale approval must never prevent compilation or weaken the schema rule.
        let fingerprint = format!("{:x}", Sha256::digest(nonblank_pattern.as_bytes()));
        let approved_nonblank_pattern = schema
            .pointer(NONBLANK_APPROVAL_PATH)
            .and_then(Value::as_str)
            .filter(|approval| *approval == fingerprint)
            .map(|_| nonblank_pattern.to_owned());
        let registry = Registry::new()
            .draft(Draft::Draft202012)
            .retriever(NoRetrieval)
            .add(SCHEMA_URI, schema.clone())
            .map_err(Error::setup)?
            .prepare()
            .map_err(Error::setup)?;
        let options = jsonschema::options()
            .with_draft(Draft::Draft202012)
            .offline()
            .with_registry(&registry);
        let root = options
            .clone()
            .with_base_uri(SCHEMA_URI)
            .build(&schema)
            .map_err(Error::compilation)?;
        let mut fields = Vec::with_capacity(FIELDS.len());
        for field in FIELDS {
            let wrapper = json!({"$ref": format!("{SCHEMA_URI}#/properties/{field}")});
            fields.push((field, options.build(&wrapper).map_err(Error::compilation)?));
        }
        Ok(Self {
            root,
            fields,
            approved_nonblank_pattern,
        })
    }

    fn validate(&self, document: &Value) -> Result<(), Error> {
        self.root.validate(document).map_err(|error| {
            Error::Content(Box::new(Violation::from_error(
                error,
                None,
                self.approved_nonblank_pattern.as_deref(),
            )))
        })
    }

    fn validate_field(&self, field: &'static str, value: &str) -> Result<(), Error> {
        let (_, validator) = self
            .fields
            .iter()
            .find(|(name, _)| *name == field)
            .ok_or_else(|| {
                Error::setup(format!(
                    "Unknown layer manifest schema field selector '{field}'"
                ))
            })?;
        validator
            .validate(&Value::String(value.to_owned()))
            .map_err(|error| {
                Error::Content(Box::new(Violation::from_error(
                    error,
                    Some(field),
                    self.approved_nonblank_pattern.as_deref(),
                )))
            })
    }
}

fn cached<'a>(
    cache: &'a OnceLock<Result<Validators, Error>>,
    source: &str,
) -> Result<&'a Validators, Error> {
    cache
        .get_or_init(|| Validators::compile(source))
        .as_ref()
        .map_err(Clone::clone)
}

/// Prepare once before guided input, so setup failures cannot become retry loops.
pub fn ensure_ready() -> Result<(), Error> {
    cached(&VALIDATORS, SCHEMA).map(|_| ())
}

/// Validate complete parsed content without file access, normalization, or field extraction.
pub fn validate(document: &Value) -> Result<(), Error> {
    cached(&VALIDATORS, SCHEMA)?.validate(document)
}

/// Validate a known property directly, without manufacturing unrelated metadata.
pub fn validate_field(field: &'static str, value: &str) -> Result<(), Error> {
    cached(&VALIDATORS, SCHEMA)?.validate_field(field, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    fn compile(schema: &Value) -> Validators {
        Validators::compile(&schema.to_string()).unwrap()
    }

    fn violation(error: Error) -> Violation {
        match error {
            Error::Content(violation) => *violation,
            other => panic!("expected content failure, got {other:?}"),
        }
    }

    #[test]
    fn schema_is_valid_draft_2020_12() {
        jsonschema::draft202012::meta::validate(&schema()).unwrap();
        ensure_ready().unwrap();
    }

    #[test]
    fn nonblank_preserves_unicode_whitespace_semantics_for_both_engines() {
        let schema = schema();
        let blanks = [
            "",
            "\t",
            "\n",
            "\u{000b}",
            "\u{000c}",
            "\r",
            " ",
            "\u{0085}",
            "\u{00a0}",
            "\u{1680}",
            "\u{2000}",
            "\u{2001}",
            "\u{2002}",
            "\u{2003}",
            "\u{2004}",
            "\u{2005}",
            "\u{2006}",
            "\u{2007}",
            "\u{2008}",
            "\u{2009}",
            "\u{200a}",
            "\u{2028}",
            "\u{2029}",
            "\u{202f}",
            "\u{205f}",
            "\u{3000}",
            " \t\r\n\u{0085}\u{00a0}\u{2003}",
        ];
        let accepted = [
            "\u{feff}",
            "\u{200b}",
            "\u{180e}",
            "\0",
            " \u{0085}visible\u{2003} ",
            " Hydragène 水 🦀 ",
        ];
        for options in [
            jsonschema::options().with_pattern_options(jsonschema::PatternOptions::fancy_regex()),
            jsonschema::options().with_pattern_options(jsonschema::PatternOptions::regex()),
        ] {
            let validator = options
                .offline()
                .with_draft(Draft::Draft202012)
                .build(&schema)
                .unwrap();
            for field in FIELDS {
                for (texts, valid) in [(&blanks[..], false), (&accepted[..], true)] {
                    for text in texts {
                        let mut document = json!({"name": "layer"});
                        document[field] = json!(text);
                        assert_eq!(validator.is_valid(&document), valid, "{field}={text:?}");
                        assert_eq!(validate(&document).is_ok(), valid, "{field}={text:?}");
                        assert_eq!(
                            validate_field(field, text).is_ok(),
                            valid,
                            "{field}={text:?}"
                        );
                        assert_eq!(document[field], json!(text));
                        if !valid {
                            let root = violation(validate(&document).unwrap_err());
                            let field_error = violation(validate_field(field, text).unwrap_err());
                            assert_eq!(root.kind, ViolationKind::NonBlank);
                            assert_eq!(field_error, root);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn complete_documents_preserve_the_metadata_contract() {
        for document in [
            json!({"name": "layer"}),
            json!({"name": "layer", "author": "person"}),
            json!({"name": "layer", "description": "purpose"}),
            json!({"name": "layer", "author": "person", "description": "purpose"}),
            json!({"name": "layer", "$schema": "https://unavailable.invalid/schema.json", "extra": {"array": [null, false, 42]}}),
            json!({"name": "layer", "$schema": "file:///unavailable/schema.json"}),
        ] {
            let original = document.clone();
            validate(&document).unwrap();
            assert_eq!(document, original);
        }
        for document in [
            Value::Null,
            json!(false),
            json!(1),
            json!("layer"),
            json!([]),
        ] {
            let failure = violation(validate(&document).unwrap_err());
            assert_eq!(failure.kind, ViolationKind::Type);
            assert_eq!(failure.instance_path, "");
            assert_eq!(failure.keyword, "type");
        }
        let missing = violation(validate(&json!({})).unwrap_err());
        assert_eq!(
            missing.kind,
            ViolationKind::Required {
                property: "name".to_owned()
            }
        );
        assert_eq!(missing.instance_path, "");
        assert_eq!(missing.schema_path, "/required");
        assert_eq!(missing.keyword, "required");
        assert!(missing.message.contains("name"));
        for field in FIELDS {
            for value in [Value::Null, json!(false), json!(1), json!([]), json!({})] {
                let mut document = json!({"name": "layer"});
                document[field] = value;
                let failure = violation(validate(&document).unwrap_err());
                assert_eq!(failure.kind, ViolationKind::Type);
                assert_eq!(failure.instance_path, format!("/{field}"));
                assert_eq!(failure.keyword, "type");
            }
        }
    }

    #[test]
    fn changed_field_constraint_applies_to_field_and_complete_content() {
        let mut schema = schema();
        schema["properties"]["name"]["maxLength"] = json!(3);
        let validators = compile(&schema);
        validators.validate_field("name", "abc").unwrap();
        validators.validate(&json!({"name": "abc"})).unwrap();
        validators.validate_field("name", "水🦀x").unwrap();
        validators.validate(&json!({"name": "水🦀x"})).unwrap();
        let field = violation(validators.validate_field("name", "abcd").unwrap_err());
        let root = violation(validators.validate(&json!({"name": "abcd"})).unwrap_err());
        assert_eq!(field, root);
        assert_eq!(field.instance_path, "/name");
        assert_eq!(field.keyword, "maxLength");
        assert_eq!(field.kind, ViolationKind::Other);
        assert!(field.message.contains('3'));
    }

    #[test]
    fn unrelated_pattern_failure_is_not_labelled_nonblank() {
        let mut schema = schema();
        schema["properties"]["name"]["pattern"] = json!("^prefix-");
        let validators = compile(&schema);
        let field = violation(validators.validate_field("name", "visible").unwrap_err());
        let root = violation(
            validators
                .validate(&json!({"name": "visible"}))
                .unwrap_err(),
        );
        assert_eq!(field, root);
        assert_eq!(field.kind, ViolationKind::Other);
        assert_eq!(field.keyword, "pattern");
        assert_eq!(field.schema_path, "/properties/name/pattern");
    }

    #[test]
    fn changed_named_pattern_without_new_approval_uses_generic_feedback() {
        let mut schema = schema();
        let approval = schema.pointer(NONBLANK_APPROVAL_PATH).unwrap().clone();
        schema["$defs"]["nonBlankString"]["pattern"] = json!("^prefix-");
        let validators = compile(&schema);
        assert!(validators.approved_nonblank_pattern.is_none());
        assert_eq!(schema.pointer(NONBLANK_APPROVAL_PATH), Some(&approval));
        for field in FIELDS {
            let mut document = json!({"name": "prefix-layer"});
            document[field] = json!("visible");
            let field_error = violation(validators.validate_field(field, "visible").unwrap_err());
            let root_error = violation(validators.validate(&document).unwrap_err());
            assert_eq!(field_error, root_error);
            assert_eq!(field_error.kind, ViolationKind::Other);
            assert_eq!(field_error.keyword, "pattern");
            assert_eq!(field_error.schema_path, NONBLANK_PATTERN_PATH);
            validators.validate_field(field, "prefix-visible").unwrap();
            document[field] = json!("prefix-visible");
            validators.validate(&document).unwrap();
        }
    }

    #[test]
    fn missing_or_stale_approval_changes_feedback_not_validity() {
        let shipped = compile(&schema());
        assert!(shipped.approved_nonblank_pattern.is_some());
        for approval in [
            None,
            Some(json!("stale")),
            Some(Value::Null),
            Some(json!(42)),
        ] {
            let mut schema = schema();
            let definition = schema["$defs"]["nonBlankString"].as_object_mut().unwrap();
            match approval {
                Some(approval) => {
                    definition.insert("x-hydragen-nonblank-pattern-sha256".to_owned(), approval);
                }
                None => {
                    definition.remove("x-hydragen-nonblank-pattern-sha256");
                }
            }
            let validators = compile(&schema);
            assert!(validators.approved_nonblank_pattern.is_none());
            for field in FIELDS {
                for text in ["", "\u{0085}", " \t\u{00a0}\u{2003}"] {
                    let mut document = json!({"name": "layer"});
                    document[field] = json!(text);
                    let approved = violation(shipped.validate_field(field, text).unwrap_err());
                    assert_eq!(approved.kind, ViolationKind::NonBlank);
                    let field_error =
                        violation(validators.validate_field(field, text).unwrap_err());
                    let root_error = violation(validators.validate(&document).unwrap_err());
                    assert_eq!(field_error, root_error);
                    assert_eq!(field_error.kind, ViolationKind::Other);
                    assert_eq!(field_error.keyword, approved.keyword);
                    assert_eq!(field_error.schema_path, approved.schema_path);
                    assert_eq!(field_error.message, approved.message);
                }
                for text in ["visible", "\u{feff}", " 水🦀 "] {
                    let mut document = json!({"name": "layer"});
                    document[field] = json!(text);
                    validators.validate_field(field, text).unwrap();
                    validators.validate(&document).unwrap();
                }
            }
        }
    }

    #[test]
    fn whole_object_constraints_remain_independent_of_field_feedback() {
        let mut schema = schema();
        schema["dependentRequired"] = json!({"author": ["description"]});
        let validators = compile(&schema);
        validators.validate_field("name", "layer").unwrap();
        validators.validate_field("author", "person").unwrap();
        let error = violation(
            validators
                .validate(&json!({"name": "layer", "author": "person"}))
                .unwrap_err(),
        );
        assert_eq!(error.keyword, "required");
        assert_eq!(
            error.kind,
            ViolationKind::Required {
                property: "description".to_owned()
            }
        );
        assert_eq!(error.instance_path, "");
        validators
            .validate(&json!({"name": "layer", "author": "person", "description": "purpose"}))
            .unwrap();
    }

    #[test]
    fn malformed_or_invalid_schema_is_an_owned_setup_failure() {
        for source in ["{", "null", r#"{"type":42}"#, r#"{"required":[42]}"#] {
            let error = Validators::compile(source).err().expect("must fail");
            assert!(matches!(error, Error::Setup { .. }));
            assert_eq!(error.clone(), error);
            assert!(
                error
                    .to_string()
                    .contains("Internal layer manifest schema error")
            );
        }
        let error = Validators::compile(r#"{"type":42}"#).err().unwrap();
        let Error::Setup {
            detail: Some(detail),
            ..
        } = error
        else {
            panic!("expected structured invalid-schema detail");
        };
        assert_eq!(detail.instance_path, "/type");
        assert!(!detail.keyword.is_empty());
        assert!(!detail.schema_path.is_empty());
        assert!(!detail.message.is_empty());
        let mut schema = schema();
        schema["properties"]["name"]["pattern"] = json!("[");
        assert!(matches!(
            Validators::compile(&schema.to_string()),
            Err(Error::Setup { .. })
        ));
    }

    #[test]
    fn external_http_and_file_references_fail_offline() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("external.schema.json");
        std::fs::write(&path, r#"{"type":"string"}"#).unwrap();
        let existing_file_uri = format!("file://{}", path.display());
        for uri in [
            "http://unavailable.invalid/schema.json",
            "https://unavailable.invalid/schema.json",
            "file:///unavailable/schema.json",
            &existing_file_uri,
        ] {
            let mut schema = schema();
            schema["properties"]["name"]["$ref"] = json!(uri);
            let error = Validators::compile(&schema.to_string())
                .err()
                .expect("must not resolve external references");
            assert!(matches!(error, Error::Setup { .. }));
            assert!(error.to_string().contains(uri));
            assert!(
                error
                    .to_string()
                    .contains("External schema retrieval is disabled")
            );
        }
    }

    #[test]
    fn content_errors_outlive_the_document_and_keep_their_source() {
        let error = {
            let document = json!({"name": "layer", "description": null});
            validate(&document).unwrap_err()
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
        let failure = violation(cloned);
        assert_eq!(failure.instance_path, "/description");
        assert_eq!(failure.keyword, "type");
        assert_eq!(failure.kind, ViolationKind::Type);
        assert!(std::error::Error::source(&error).is_some());
        assert!(error.to_string().contains("/description (type)"));
    }

    #[test]
    fn unavailable_definitions_and_unknown_selectors_fail_explicitly() {
        for field in FIELDS {
            let mut schema = schema();
            schema["properties"].as_object_mut().unwrap().remove(field);
            let error = Validators::compile(&schema.to_string())
                .err()
                .expect("missing field definition");
            assert!(matches!(error, Error::Setup { .. }));
            assert!(error.to_string().contains(&format!("/properties/{field}")));
        }
        let mut missing_target = schema();
        missing_target["properties"]["name"]["$ref"] = json!("#/$defs/missing");
        assert!(matches!(
            Validators::compile(&missing_target.to_string()),
            Err(Error::Setup { .. })
        ));
        let mut missing_nonblank = schema();
        missing_nonblank["$defs"]
            .as_object_mut()
            .unwrap()
            .remove("nonBlankString");
        assert!(matches!(
            Validators::compile(&missing_nonblank.to_string()),
            Err(Error::Setup { .. })
        ));
        for field in ["unknown", "", "name/type"] {
            let error = validate_field(field, "valid").unwrap_err();
            assert!(matches!(error, Error::Setup { .. }));
            assert!(
                error
                    .to_string()
                    .contains("Unknown layer manifest schema field selector")
            );
        }
    }

    #[test]
    fn successful_initialization_is_cached() {
        let cache = OnceLock::new();
        let first = cached(&cache, SCHEMA).unwrap();
        // An invalid second source cannot replace or recompile a successful result.
        let second = cached(&cache, "not JSON").unwrap();
        assert!(std::ptr::eq(first, second));
        second.validate_field("name", "layer").unwrap();
        assert!(std::ptr::eq(
            cached(&VALIDATORS, SCHEMA).unwrap(),
            cached(&VALIDATORS, SCHEMA).unwrap()
        ));
    }

    #[test]
    fn initialization_failure_is_cached() {
        let mut external = schema();
        external["properties"]["name"]["$ref"] = json!("https://unavailable.invalid/schema.json");
        for source in ["not JSON", r#"{"type":42}"#, &external.to_string()] {
            let cache = OnceLock::new();
            let first = cached(&cache, source).err().expect("invalid schema");
            // A valid second source must not retry a failed initialization.
            let second = cached(&cache, SCHEMA).err().expect("cached failure");
            assert_eq!(first, second);
            assert!(matches!(first, Error::Setup { .. }));
        }
    }
}
