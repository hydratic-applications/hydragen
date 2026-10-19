//! Shared schema and metadata facts for init, loading and add diagnostics.
//! This adapts existing validation results; it never validates input itself.

use crate::diagnostic_body::Diagnostic;
use hydragen::diagnostic_text::display;
use hydragen::layer_manifest::{MetadataError, schema};

pub(super) const DECLARED_REFERENCE_CORRECTION: &str = "Use an explicit local path beginning with ./ or ../, an absolute path, or a published reference with an exact version, such as '@scope/shared-config@1.2.3'.";

/// `path` is already escaped presentation text, supplied by the path-aware adapter.
pub(super) fn metadata_diagnostic(source: &MetadataError, path: &str) -> Diagnostic {
    let mut model = Diagnostic::new(format!("Invalid layer manifest '{path}'."));
    if let MetadataError::Schema(schema::Error::Content(violation)) = source
        && declared_reference_location(violation)
    {
        model.summary = format!("Invalid dependency reference in layer manifest '{}'.", path);
        model
            .details
            .push(format!("Field: {}", display(&violation.instance_path)));
        model.correction = Some(DECLARED_REFERENCE_CORRECTION.into());
    } else {
        metadata_details(source, &mut model);
    }
    if matches!(source, MetadataError::Schema(schema::Error::Setup { .. })) {
        model.summary = "Internal layer manifest schema error.".into();
        model.details.insert(0, format!("Layer manifest: '{path}'"));
    }
    model
}

pub(super) fn known_reference_rule(violation: &schema::Violation) -> bool {
    violation.schema_path == "/$defs/reference/oneOf" && violation.keyword == "oneOf"
}

fn declared_reference_location(violation: &schema::Violation) -> bool {
    // Recognise the known direct array-item location in a schema diagnostic,
    // not another grammar for dependency values. Unknown pointer shapes keep
    // their original constraint details instead of borrowing this explanation.
    known_reference_rule(violation)
        && violation
            .instance_path
            .strip_prefix("/dependencies/")
            .is_some_and(|index| {
                index
                    .parse::<usize>()
                    .is_ok_and(|value| value.to_string() == index)
            })
}

pub(super) fn metadata_details(source: &MetadataError, model: &mut Diagnostic) {
    let (detail, correction) = match source {
        MetadataError::NotObject => (
            "Layer manifest must be one object/mapping document.".into(),
            "Use one object/mapping document for the layer manifest.".into(),
        ),
        MetadataError::MissingField { field } => (
            format!("Missing required metadata field '{}'.", display(field)),
            format!("Add the required '{}' field to the selected layer manifest.", display(field)),
        ),
        MetadataError::NotString { field } => (
            format!("Metadata field '{}' must be a string.", display(field)),
            format!("Set '{}' to a string, or omit it only if it is optional.", display(field)),
        ),
        MetadataError::Blank { field } => (
            format!("Metadata field '{}' must contain at least one non-whitespace character.", display(field)),
            format!("Provide a non-blank '{}' string, or omit it only if it is optional.", display(field)),
        ),
        MetadataError::InvalidName { guidance } => (
            format!("Metadata field 'name' is invalid. {}", display(guidance)),
            "Correct the 'name' field using the constraint above; names are not rewritten automatically.".into(),
        ),
        MetadataError::Schema(source) => { schema_details(source, model); return; }
    };
    model.details.push(detail);
    model.correction = Some(correction);
}

pub(super) fn schema_details(source: &schema::Error, model: &mut Diagnostic) {
    match source {
        schema::Error::Setup { message, detail } => {
            model.details.push(format!("Details: {}", display(message)));
            if let Some(detail) = detail {
                violation_details(detail, model);
            }
            model.correction = Some(
                "Report this internal error; changing valid input will not resolve it.".into(),
            );
        }
        schema::Error::Content(violation) => {
            violation_details(violation, model);
            model.correction = Some(match &violation.kind {
                schema::ViolationKind::NonBlank => "Provide a string containing at least one non-whitespace character at the reported field.".into(),
                schema::ViolationKind::Name { guidance } => format!("Correct the layer name: {}", display(guidance)),
                schema::ViolationKind::Type => "Use the value type required by the reported schema constraint.".into(),
                schema::ViolationKind::Required { property } => format!("Add the required '{}' property at the reported location.", display(property)),
                schema::ViolationKind::Other => "Correct the value at the reported location using the schema constraint above.".into(),
            });
        }
    }
}

fn violation_details(violation: &schema::Violation, model: &mut Diagnostic) {
    // Violation's Display escapes its instance path, keyword and opaque message.
    // Retain the schema location too; never dump a Debug representation of fields.
    model.details.push(violation.to_string());
    model.details.push(format!(
        "Schema rule: '{}'",
        display(&violation.schema_path)
    ));
}
