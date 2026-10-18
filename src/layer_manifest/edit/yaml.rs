//! Insertion-only YAML edits. The caller owns guarded decoding and candidate proof.

use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;

use serde_saphyr::Budget;
use serde_saphyr::granit_parser::{
    self as parser, Event, ScalarStyle, Span, StructureStyle, TokenType,
};

pub(super) fn append(source: &str, references: &[&str]) -> Result<String, String> {
    if references.is_empty() {
        return Ok(source.to_owned());
    }
    let document = Document::parse(source, &Budget::default())?;
    let root = &document.nodes[document.root];
    let Kind::Mapping(style) = root.kind else {
        return Err("The YAML layer manifest must have a mapping at its root.".into());
    };
    let literals = references
        .iter()
        .map(|reference| quoted(reference))
        .collect::<Result<Vec<_>, _>>()?;
    let dependencies = root
        .children
        .chunks_exact(2)
        .find_map(|pair| (document.scalar_key(pair[0]) == Some("dependencies")).then_some(pair[1]));
    let (position, addition) = if let Some(index) = dependencies {
        let node = &document.nodes[index];
        if matches!(node.kind, Kind::Alias(_)) {
            return Err("The YAML dependencies list is an alias; replace it with an explicit sequence before adding dependencies.".into());
        }
        let Kind::Sequence(style) = node.kind else {
            return Err("The YAML dependencies value must be a sequence.".into());
        };
        if node.anchor != 0
            && document
                .nodes
                .iter()
                .any(|other| matches!(other.kind, Kind::Alias(anchor) if anchor == node.anchor))
        {
            return Err("The YAML dependencies sequence has an anchor used by an alias; adding entries would also change another value. Use an unshared explicit sequence first.".into());
        }
        match style {
            StructureStyle::Flow => document.flow_insertion(node, &literals.join(", "))?,
            StructureStyle::Block => document.block_insertion(source, node, &literals)?,
        }
    } else {
        let declaration = format!("dependencies: [{}]", literals.join(", "));
        match style {
            StructureStyle::Flow => document.flow_insertion(root, &declaration)?,
            StructureStyle::Block => document.absent_block_insertion(source, root, &declaration)?,
        }
    };
    let mut candidate = String::with_capacity(source.len() + addition.len());
    candidate.push_str(&source[..position]);
    candidate.push_str(&addition);
    candidate.push_str(&source[position..]);
    Ok(candidate)
}

enum Kind<'source> {
    Scalar(Cow<'source, str>, ScalarStyle),
    Alias(usize),
    Sequence(StructureStyle),
    Mapping(StructureStyle),
}

struct Node<'source> {
    kind: Kind<'source>,
    anchor: usize,
    start: Span,
    end: Span,
    children: Vec<usize>,
}

struct Token {
    kind: Syntax,
    range: Range<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum Syntax {
    Open,
    Close,
    Comma,
    Dash,
    Value,
    Other,
}

struct Document<'source> {
    nodes: Vec<Node<'source>>,
    root: usize,
    tokens: Vec<Token>,
    comments: Vec<Range<usize>>,
    scalar_anchors: HashMap<usize, usize>,
}

impl<'source> Document<'source> {
    fn scalar_key(&self, index: usize) -> Option<&str> {
        match &self.nodes[index].kind {
            Kind::Scalar(value, _) => Some(value),
            Kind::Alias(id) => self.scalar_anchors.get(id).and_then(|&index| {
                if let Kind::Scalar(value, _) = &self.nodes[index].kind {
                    Some(value.as_ref())
                } else {
                    None
                }
            }),
            _ => None,
        }
    }

    fn parse(source: &'source str, budget: &Budget) -> Result<Self, String> {
        let options = parser::options! {
            emit_comments: true,
            simple_key_max_lookahead: budget.simple_key_max_lookahead,
            flow_nesting_limit: budget.flow_nesting_limit,
        };
        let mut nodes: Vec<Node<'source>> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        let mut root = None;
        let mut documents = 0;
        let mut scalar_bytes = 0usize;
        let mut scalar_anchors = HashMap::new();
        for (count, event) in
            parser::Parser::new_from_str_with_options(source, options.clone()).enumerate()
        {
            if count >= budget.max_events {
                return Err("YAML editing exceeded the parser event budget.".into());
            }
            let (event, span) = event.map_err(|error| error.to_string())?;
            byte_range(source, span)?;
            let anchor = event.anchor_id().unwrap_or(0);
            let kind = match event {
                Event::DocumentStart(..) => {
                    documents += 1;
                    if documents > 1 {
                        return Err("Only one YAML document can be edited.".into());
                    }
                    continue;
                }
                Event::SequenceEnd | Event::MappingEnd => {
                    let index = stack.pop().ok_or("Unbalanced YAML collection events.")?;
                    nodes[index].end = span;
                    continue;
                }
                Event::Scalar(value, style, ..) => {
                    scalar_bytes = scalar_bytes.saturating_add(value.len());
                    if scalar_bytes > budget.max_total_scalar_bytes {
                        return Err("YAML editing exceeded the parser scalar-byte budget.".into());
                    }
                    Kind::Scalar(value, style)
                }
                Event::Alias(id) => Kind::Alias(id),
                Event::SequenceStart(style, ..) => Kind::Sequence(style),
                Event::MappingStart(style, ..) => Kind::Mapping(style),
                _ => continue,
            };
            let collection = matches!(kind, Kind::Sequence(_) | Kind::Mapping(_));
            let index = nodes.len();
            if anchor != 0 && matches!(kind, Kind::Scalar(..)) {
                scalar_anchors.insert(anchor, index);
            }
            if let Some(&parent) = stack.last() {
                nodes[parent].children.push(index);
            } else if root.replace(index).is_some() {
                return Err("Only one YAML root can be edited.".into());
            }
            nodes.push(Node {
                kind,
                anchor,
                start: span,
                end: span,
                children: Vec::new(),
            });
            if collection {
                stack.push(index);
                if stack.len() > budget.max_depth {
                    return Err("YAML editing exceeded the parser depth budget.".into());
                }
            }
        }
        if !stack.is_empty() {
            return Err("Unclosed YAML collection events.".into());
        }
        let root = root.ok_or("The YAML layer manifest has no root mapping.")?;
        let mut tokens = Vec::new();
        let mut comments = Vec::new();
        // Scanner syntax includes property and key/value tokens in addition to
        // parser events. Bound this independent pass too, without alias expansion.
        let token_budget = budget.max_events.saturating_mul(6).saturating_add(16);
        for (count, token) in
            parser::Scanner::with_options(parser::StrInput::new(source), options).enumerate()
        {
            if count >= token_budget {
                return Err("YAML editing exceeded the scanner token budget.".into());
            }
            let token = token.map_err(|error| error.to_string())?;
            let range = byte_range(source, token.span())?;
            let kind = match token.token_type() {
                TokenType::FlowSequenceStart | TokenType::FlowMappingStart => Syntax::Open,
                TokenType::FlowSequenceEnd | TokenType::FlowMappingEnd => Syntax::Close,
                TokenType::FlowEntry => Syntax::Comma,
                TokenType::BlockEntry => Syntax::Dash,
                TokenType::Value => Syntax::Value,
                TokenType::Comment(_) => {
                    comments.push(range);
                    continue;
                }
                _ => Syntax::Other,
            };
            tokens.push(Token { kind, range });
        }
        Ok(Self {
            nodes,
            root,
            tokens,
            comments,
            scalar_anchors,
        })
    }

    fn absent_block_insertion(
        &self,
        source: &str,
        root: &Node<'_>,
        declaration: &str,
    ) -> Result<(usize, String), String> {
        // Keep the first field together with its leading own-line comments.
        // Inserting at the key itself would make those comments describe the
        // new dependencies declaration instead. Scanner ranges distinguish
        // real comments from '#' inside scalars and inline property comments.
        let start = byte_range(source, root.start)?.start;
        let indent = " ".repeat(root.start.start.col());
        let mut boundary = line_start(source, start);
        let mut position = start;
        for comment in self.comments.iter().rev() {
            if comment.end > boundary {
                continue;
            }
            let line = line_start(source, comment.start);
            // A stream BOM must remain at the beginning of the source, not
            // between the newly inserted field and the original first field.
            let line = if line == 0 && source.starts_with('\u{feff}') {
                '\u{feff}'.len_utf8()
            } else {
                line
            };
            if !source[comment.end..boundary]
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                || !source[line..comment.start]
                    .bytes()
                    .all(|byte| matches!(byte, b' ' | b'\t'))
            {
                break;
            }
            position = line;
            boundary = line;
        }
        let eol = newline(source);
        if position == start {
            // With no leading comment group, retain the parser's first-key
            // boundary. Root anchors/tags and document markers before it stay
            // attached to the mapping, rather than to a newly introduced node.
            Ok((start, format!("{declaration}{eol}{indent}")))
        } else {
            Ok((position, format!("{indent}{declaration}{eol}")))
        }
    }

    fn flow_insertion(&self, node: &Node<'_>, addition: &str) -> Result<(usize, String), String> {
        let start = node
            .start
            .byte_range()
            .ok_or("YAML flow collection has no byte range.")?
            .start;
        let opening = self
            .tokens
            .iter()
            .position(|token| token.kind == Syntax::Open && token.range.start == start)
            .ok_or("Cannot locate the YAML flow collection's opening token.")?;
        let mut depth = 0usize;
        for (offset, token) in self.tokens[opening..].iter().enumerate() {
            match token.kind {
                Syntax::Open => depth += 1,
                Syntax::Close => {
                    depth -= 1;
                    if depth == 0 {
                        // The scanner, unlike collection-end events, identifies
                        // the actual closing delimiter. Comments are retained in
                        // their original positions, including inline comments.
                        let previous = &self.tokens[opening + offset - 1];
                        let separator =
                            if node.children.is_empty() || previous.kind == Syntax::Comma {
                                ""
                            } else {
                                ", "
                            };
                        return Ok((token.range.start, format!("{separator}{addition}")));
                    }
                }
                _ => {}
            }
        }
        Err("Cannot locate the YAML flow collection's closing token.".into())
    }

    fn block_insertion(
        &self,
        source: &str,
        node: &Node<'_>,
        literals: &[String],
    ) -> Result<(usize, String), String> {
        let start = byte_range(source, node.start)?.start;
        let dash = self
            .tokens
            .iter()
            .find(|token| token.kind == Syntax::Dash && token.range.start >= start)
            .ok_or("Cannot locate the YAML dependencies sequence's first dash.")?;
        // Granit's BlockEntry marker is *after* the dash and its following
        // whitespace. The zero-width BlockSequenceStart may instead be before
        // the dash; indentless sequences have no such scanner start token.
        // Recover only this scanner-identified dash, never search YAML content.
        let line = line_start(source, dash.range.start);
        let prefix = source[line..dash.range.start].trim_end_matches([' ', '\t']);
        let prefix = prefix
            .strip_suffix('-')
            .ok_or("Cannot identify the YAML dependencies dash before its entry token.")?;
        let dash_start = line + prefix.len();
        // An explicit mapping key can introduce its sequence compactly as
        // `: - old`. That dash's actual column, not the value indicator's
        // indentation, determines the column of subsequent sequence entries.
        let compact_value = self.tokens.iter().any(|token| {
            token.kind == Syntax::Value
                && token.range.start >= line
                && token.range.start < dash_start
                && source.get(token.range.start..token.range.start + 1) == Some(":")
                && source[token.range.start + 1..dash_start]
                    .bytes()
                    .all(|byte| matches!(byte, b' ' | b'\t'))
        });
        if !prefix.bytes().all(|byte| byte == b' ') && !compact_value {
            return Err("Cannot identify the indentation of the YAML dependencies dash.".into());
        }
        let indent = prefix.chars().count();
        let last = &self.nodes[*node.children.last().ok_or("Empty YAML block sequence.")?];
        if !matches!(last.kind, Kind::Scalar(..) | Kind::Alias(_)) {
            return Err("YAML dependencies must contain scalar references.".into());
        }
        let end = byte_range(source, last.end)?.end;
        let block_scalar = matches!(
            last.kind,
            Kind::Scalar(_, ScalarStyle::Literal | ScalarStyle::Folded)
        );
        let line = line_start(source, end);
        let line_boundary = block_scalar
            && source[line..end]
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t'));
        // A block scalar's span can end at the next key's line, whereas a
        // quoted/plain scalar ends before its inline comment. Never insert at
        // the collection-end event: indentless ends may point *into* that key.
        let position = if line_boundary {
            line
        } else {
            next_line(source, end)
        };
        let position = self.trailing_item_comment_end(source, position, indent);
        let eol = newline(source);
        let mut addition = String::new();
        if position > 0 && !source[..position].ends_with(['\r', '\n']) {
            addition.push_str(eol);
        }
        for (index, literal) in literals.iter().enumerate() {
            if index > 0 {
                addition.push_str(eol);
            }
            addition.push_str(&" ".repeat(indent));
            addition.push_str("- ");
            addition.push_str(literal);
        }
        if position < source.len() || source.ends_with(['\r', '\n']) {
            addition.push_str(eol);
        }
        Ok((position, addition))
    }

    fn trailing_item_comment_end(&self, source: &str, mut position: usize, indent: usize) -> usize {
        for comment in &self.comments {
            if comment.start < position {
                continue;
            }
            let line = line_start(source, comment.start);
            let prefix = &source[line..comment.start];
            // More-indented own-line comments belong to the previous item.
            // Keep that item and its complete comment group together, allowing
            // blank lines between comments. Sequence-level footers and comments
            // for following fields stay after the new entries: the first such
            // comment, or any intervening data, ends the attached group.
            if line < position
                || !source[position..line]
                    .bytes()
                    .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                || prefix.len() <= indent
                || !prefix.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
            {
                break;
            }
            position = next_line(source, comment.end);
        }
        position
    }
}

fn byte_range(source: &str, span: Span) -> Result<Range<usize>, String> {
    let range = span
        .byte_range()
        .ok_or("YAML editor requires byte ranges from string-backed parser input.")?;
    if source.get(range.clone()).is_none() {
        return Err("YAML parser supplied an invalid source byte range.".into());
    }
    Ok(range)
}

fn line_start(source: &str, position: usize) -> usize {
    source[..position]
        .rfind(['\r', '\n'])
        .map_or(0, |index| index + 1)
}

fn next_line(source: &str, position: usize) -> usize {
    source[position..]
        .find(['\r', '\n'])
        .map_or(source.len(), |relative| {
            let end = position + relative + 1;
            if source.as_bytes()[end - 1] == b'\r' && source.as_bytes().get(end) == Some(&b'\n') {
                end + 1
            } else {
                end
            }
        })
}

fn newline(source: &str) -> &'static str {
    match source.find(['\r', '\n']) {
        Some(index) if source.as_bytes()[index] == b'\r' => {
            if source.as_bytes().get(index + 1) == Some(&b'\n') {
                "\r\n"
            } else {
                "\r"
            }
        }
        _ => "\n",
    }
}

fn quoted(reference: &str) -> Result<String, String> {
    let json = serde_json::to_string(reference).map_err(|error| error.to_string())?;
    let mut literal = String::with_capacity(json.len());
    for character in json.chars() {
        // YAML's printable source set is narrower than JSON's. Escaping C1
        // controls and unusual Unicode separators in only the new literal
        // preserves the reference's exact decoded text without ambiguity.
        if matches!(
            character,
            '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}' | '\u{fffe}' | '\u{ffff}'
        ) {
            use std::fmt::Write;
            write!(literal, "\\u{:04x}", u32::from(character))
                .map_err(|error| error.to_string())?;
        } else {
            literal.push(character);
        }
    }
    Ok(literal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn check(source: &str, references: &[&str]) -> String {
        let original: Value = serde_saphyr::from_str(source).unwrap();
        let output = append(source, references).unwrap();
        let mut expected = original;
        let array = expected
            .as_object_mut()
            .unwrap()
            .entry("dependencies")
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .unwrap();
        array.extend(
            references
                .iter()
                .map(|value| Value::String((*value).into())),
        );
        let actual: Value =
            serde_saphyr::from_str(&output).unwrap_or_else(|error| panic!("{output:?}: {error}"));
        assert_eq!(actual, expected, "{output:?}");
        // Independently require exactly one insertion: every existing source
        // byte, including whitespace and comments, must remain in order.
        let common = source
            .bytes()
            .zip(output.bytes())
            .take_while(|(left, right)| left == right)
            .count();
        assert!(output[common..].ends_with(&source[common..]), "{output:?}");
        output
    }

    fn check_guarded_exact(source: &str, expected: &str) {
        use crate::layer_manifest::edit::append_dependencies;
        use crate::layer_manifest::{LayerManifestFormat, parse_source};
        use std::path::Path;

        for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
            let path = Path::new(format.filename());
            let original = parse_source(path, format, source.to_owned()).unwrap();
            let candidate = append_dependencies(&original, &["../new"])
                .unwrap_or_else(|error| panic!("{source:?}: {error}"));
            assert_eq!(candidate, expected, "{format}: {source:?}");
            let mut intended = original.document;
            intended["dependencies"]
                .as_array_mut()
                .unwrap()
                .push(Value::String("../new".into()));
            let decoded = parse_source(path, format, candidate).unwrap();
            assert_eq!(decoded.document, intended);
        }
        assert_eq!(check(source, &["../new"]), expected);
    }

    #[test]
    fn block_trailing_item_comment_regression() {
        check_guarded_exact(
            "name: app\ndependencies:\n  - ../old\n    # Required by old, not new\ncustom: unchanged\n",
            "name: app\ndependencies:\n  - ../old\n    # Required by old, not new\n  - \"../new\"\ncustom: unchanged\n",
        );
        check_guarded_exact(
            "name: app\ndependencies:\n- &old '../old' # inline old\n  # Required by old\n# Following alias field\ncustom: *old\n",
            "name: app\ndependencies:\n- &old '../old' # inline old\n  # Required by old\n- \"../new\"\n# Following alias field\ncustom: *old\n",
        );
    }

    #[test]
    fn block_compact_explicit_key_regression() {
        check_guarded_exact(
            "name: app\n? dependencies\n: - ../old\ncustom: unchanged\n",
            "name: app\n? dependencies\n: - ../old\n  - \"../new\"\ncustom: unchanged\n",
        );
    }

    #[test]
    fn block_trailing_item_comments_keep_adjacency_across_scalar_styles_and_boundaries() {
        for item in [
            "../old # inline old",
            "'../old' # quoted old",
            "\"../old\" # double quoted old",
            "&old ../old # anchored old",
            "|- # literal header\n      ../old",
            ">- # folded header\n      ../old",
        ] {
            for gap in ["", "\n", "  \n"] {
                let attached = format!(
                    "name: app\ndependencies:\n  - ../before # earlier item\n  - {item}\n{gap}    # Required by old, not new\n{gap}    # More detail about old"
                );
                for suffix in [
                    "\ncustom: unchanged\n",
                    "\n# Following field\ncustom: unchanged",
                    "\n  # Sequence footer\n# Following field\ncustom: unchanged\n",
                    "",
                    "\n",
                    "\n\n  # Sequence footer at EOF",
                    "\n... # End document\n# Outside document\n",
                ] {
                    for eol in ["\n", "\r\n"] {
                        let source = format!("{attached}{suffix}").replace('\n', eol);
                        let expected =
                            format!("{attached}\n  - \"../new\"{suffix}").replace('\n', eol);
                        check_guarded_exact(&source, &expected);
                    }
                }
            }
        }
    }

    #[test]
    fn block_compact_sequences_support_key_properties_items_comments_and_indentation() {
        for key in [
            "dependencies",
            "'dependencies'",
            "\"depend\\u0065ncies\"",
            "!!str dependencies",
            "&key dependencies",
            "!!str &key 'dependencies'",
        ] {
            for root_indent in ["", "  "] {
                for item in ["../old", "'../old'", "&old ../old", "!!str \"../old\""] {
                    for more in [
                        String::new(),
                        format!("\n{root_indent}  - ../second # second inline"),
                    ] {
                        let attached = format!(
                            "{root_indent}name: app\n{root_indent}? {key}\n{root_indent}: - {item} # first inline{more}\n{root_indent}    # Last existing item detail"
                        );
                        for suffix in [
                            String::new(),
                            "\n".into(),
                            format!("\n{root_indent}custom: unchanged\n"),
                            format!(
                                "\n{root_indent}  # Sequence footer\n{root_indent}# Following field\n{root_indent}custom: unchanged"
                            ),
                        ] {
                            for eol in ["\n", "\r\n"] {
                                let source = format!("{attached}{suffix}").replace('\n', eol);
                                let expected =
                                    format!("{attached}\n{root_indent}  - \"../new\"{suffix}")
                                        .replace('\n', eol);
                                check_guarded_exact(&source, &expected);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn block_sequences_preserve_inline_and_standalone_comments() {
        for source in [
            "# before\nname: app\ndependencies: # list\n  - ../old # keep old\n  # keep trailing\ncustom: yes # custom\n",
            "name: app\ndependencies:\n- '../old' # keep old\n# before custom\ncustom: [a, b]\n",
            "name: app\ndependencies:\n    - ../old # no final newline",
            "name: app\ndependencies:\n  - '../old'\n  - \"../two\" # second\n... # end\n# outside\n",
        ] {
            let output = check(source, &["../new", "../other"]);
            for fragment in [
                "# keep old",
                "# keep trailing",
                "# before custom",
                "# custom",
                "# end",
                "# outside",
            ] {
                assert_eq!(
                    output.matches(fragment).count(),
                    source.matches(fragment).count()
                );
            }
            assert_eq!(output.ends_with('\n'), source.ends_with('\n'));
        }
    }

    #[test]
    fn flow_sequences_preserve_comments_commas_and_closing_delimiters() {
        for source in [
            "name: app\ndependencies: [] # empty\n",
            "name: app\ndependencies: [ # first\n] # last",
            "name: app\ndependencies: [../old] # right\n",
            "name: app\ndependencies: [../old, ]\n",
            "name: app\ndependencies: [../old # existing inline\n  # closing comment\n  ] # after\n",
            "name: app\ndependencies: [../old, # existing inline\n  ]\n",
            "{name: app, dependencies: [../old # old\n], custom: {nested: [one, two]}}",
        ] {
            check(source, &["../new"]);
        }
    }

    #[test]
    fn absent_block_dependencies_keep_first_field_comment_attachment() {
        for (source, expected) in [
            (
                "# Name of this app\nname: app\ncustom: true\n",
                "dependencies: [\"../new\"]\n# Name of this app\nname: app\ncustom: true\n",
            ),
            (
                "# Header\n\n# Name of this app\n# Another name detail\nname: app # name inline\n# Custom flag\ncustom: true\n",
                "dependencies: [\"../new\"]\n# Header\n\n# Name of this app\n# Another name detail\nname: app # name inline\n# Custom flag\ncustom: true\n",
            ),
            (
                "---\n# Name of this app\nname: app\n...\n# Outside document\n",
                "---\ndependencies: [\"../new\"]\n# Name of this app\nname: app\n...\n# Outside document\n",
            ),
            (
                "# Stream header\n%YAML 1.2\n--- # Document\n# Name of this app\nname: app\n... # End\n",
                "# Stream header\n%YAML 1.2\n--- # Document\ndependencies: [\"../new\"]\n# Name of this app\nname: app\n... # End\n",
            ),
            (
                "  # Name of this app\n  name: app\n  custom: true\n",
                "  dependencies: [\"../new\"]\n  # Name of this app\n  name: app\n  custom: true\n",
            ),
            (
                "# 💧 Name\r\n  # é detail\r\n  name: app\r\n  custom: true",
                "  dependencies: [\"../new\"]\r\n# 💧 Name\r\n  # é detail\r\n  name: app\r\n  custom: true",
            ),
            (
                "\u{feff}# Name of this app\nname: app",
                "\u{feff}dependencies: [\"../new\"]\n# Name of this app\nname: app",
            ),
        ] {
            assert_eq!(check(source, &["../new"]), expected);
        }
    }

    #[test]
    fn absent_block_comment_boundary_keeps_root_and_key_properties_attached() {
        for (source, expected) in [
            (
                "&root\n# Name\nname: app\n",
                "&root\ndependencies: [\"../new\"]\n# Name\nname: app\n",
            ),
            (
                "!!map\n# Name\nname: app\n",
                "!!map\ndependencies: [\"../new\"]\n# Name\nname: app\n",
            ),
            (
                "# Root mapping\n&root !!map # Root inline\n# Name\nname: app\n",
                "# Root mapping\n&root !!map # Root inline\ndependencies: [\"../new\"]\n# Name\nname: app\n",
            ),
            (
                "--- &root !!map\n# Name\nname: app\n...\n",
                "--- &root !!map\ndependencies: [\"../new\"]\n# Name\nname: app\n...\n",
            ),
            (
                "%TAG !e! tag:yaml.org,2002:\n--- !e!map &root\n  # Name\n  name: app\n...\n",
                "%TAG !e! tag:yaml.org,2002:\n--- !e!map &root\n  dependencies: [\"../new\"]\n  # Name\n  name: app\n...\n",
            ),
            (
                "# Name\n&key name: app\ncustom: *key\n",
                "dependencies: [\"../new\"]\n# Name\n&key name: app\ncustom: *key\n",
            ),
            (
                "# Name\n!!str name: app\ncustom: true\n",
                "dependencies: [\"../new\"]\n# Name\n!!str name: app\ncustom: true\n",
            ),
            (
                "# Name\n? &key name\n: app\ncustom: *key\n",
                "dependencies: [\"../new\"]\n# Name\n? &key name\n: app\ncustom: *key\n",
            ),
        ] {
            assert_eq!(check(source, &["../new"]), expected);
        }
    }

    #[test]
    fn absent_dependencies_support_flow_and_block_roots_and_document_markers() {
        for source in [
            "name: app",
            "# before\nname: app # name\n# after",
            "%YAML 1.2\n---\n# leading\nname: app\n... # finished\n# after doc\n",
            "  name: app\n  custom: value\n",
            "? name\n: app\n? custom\n: value\n",
            "{name: app}",
            "{name: app, # name comment\n custom: {nested: [1, 2]}, # closing\n}\n...\n# outside",
            "--- {name: app}\n...\n",
            "name: app\ncustom: |+\n  keep all\n\n\n",
        ] {
            check(source, &["../new"]);
        }
    }

    #[test]
    fn unicode_escaped_keys_and_new_literals_use_byte_offsets() {
        for source in [
            "name: app\ncustom: '💧 é'\n\"depend\\u0065ncies\": ['../old'] # 💧\n",
            "{custom: '💧 é', name: app, 'dependencies': []}",
            "# 💧 é\nname: app\ndependencies:\n  - \"../é💧\" # old 💧\ncustom: intact\n",
            "# 💧\n\"na\\u006de\": app",
        ] {
            check(
                source,
                &[
                    "../é💧\\quoted\"\n\t",
                    "../a\u{85}\u{2028}\u{2029}\u{7f}\u{ffff}",
                ],
            );
        }
    }

    #[test]
    fn crlf_and_final_newline_policy_are_preserved() {
        for source in [
            "name: app\r\ndependencies:\r\n  - ../old # old\r\ncustom: intact\r\n",
            "name: app\r\ndependencies:\r\n- ../old # old",
            "%YAML 1.2\r\n---\r\nname: app\r\n...\r\n# after",
            "name: app\r\ndependencies: [../old, # old\r\n]\r\n",
        ] {
            let output = check(source, &["../new", "../second"]);
            assert!(!output.replace("\r\n", "").contains('\n'));
            assert_eq!(source.ends_with('\n'), output.ends_with('\n'));
        }
    }

    #[test]
    fn safe_anchors_remain_supported() {
        for source in [
            "name: app\ncustom: &shared [one, two]\nother: *shared\ndependencies: [../old]\n",
            "name: app\ndependencies: &unshared [../old]\n",
            "name: app\ndependencies: &unshared\n  - ../old # old\n",
            "name: app\ndependencies:\n- &old ../old # anchored scalar\ncustom: *old\n",
            "name: app\ncustom: &old ../old\ndependencies: [*old]\n",
            "name: app\ndependencies: [&old ../old]\ncustom: *old\n",
            "&root\nname: app\ncustom: &shared [one]\nother: *shared\n",
            "&key name: app\ncustom: *key\n",
            "!!str name: app\ncustom: value\n",
            "? &key name\n: app\ncustom: *key\n",
            "name: app\ndependencies: &shared [../old]\ncustom: &shared [other]\nother: *shared\n",
        ] {
            check(source, &["../new"]);
        }
    }

    #[test]
    fn unsafe_dependency_alias_and_shared_sequence_have_specific_refusals() {
        for (source, message) in [
            (
                "name: app\ncustom: &shared [../old]\ndependencies: *shared\n",
                "list is an alias",
            ),
            (
                "name: app\ndependencies: &shared [../old]\ncustom: *shared\n",
                "anchor used by an alias",
            ),
            (
                "name: app\ndependencies: &shared\n- ../old\ncustom: {nested: *shared}\n",
                "anchor used by an alias",
            ),
        ] {
            assert!(append(source, &["../new"]).unwrap_err().contains(message));
        }
    }

    #[test]
    fn block_scalar_reference_and_following_mapping_are_not_split() {
        for source in [
            "name: app\ndependencies:\n- |-\n  ../old\ncustom: intact\n",
            "name: app\ndependencies:\n  - >- # scalar comment\n    ../old\n  # sequence comment\ncustom: intact\n",
            "name: app\ndependencies:\n- |-\n  ../old",
            "  name: app\n  dependencies:\n    - |-\n      ../old\n  custom: intact\n",
            "name: app\ndependencies:\n- |\n  ../old\ncustom: intact\n",
            "name: app\ndependencies:\n- |+\n  ../old\n\n\ncustom: intact\n",
            "name: app\ndependencies:\n- >-\n  ../old\n  continued\ncustom: intact\n",
        ] {
            check(source, &["../new"]);
        }
    }

    #[test]
    fn tagged_and_aliased_keys_and_multiline_scalars_preserve_structure() {
        for source in [
            "%TAG !e! tag:yaml.org,2002:\n---\nname: app\n!e!str dependencies: !e!seq [!e!str ../old] # keep\n...\n",
            "name: app\ncustom: &key dependencies\n*key : [../old]\n",
            "name: app\ncustom: &key dependencies\n? *key\n: [../old]\n",
            "name: app\n&key dependencies:\n  - ../old\ncustom: *key\n",
            "name: app\ndependencies:\n - \"../old\n   continued\" # multiline\ncustom: intact\n",
            "name: app\ndependencies:\n- ../old\n  continued # multiline plain\ncustom: intact\n",
            "\u{feff}name: app\ndependencies:\n- ../old # BOM\n",
            "!!map\nname: app\ncustom: value\n",
            "name: app\rdependencies:\r- ../old # old\rcustom: intact\r...\r# after",
        ] {
            check(source, &["../new"]);
        }
    }

    #[test]
    fn eof_block_scalar_chomping() {
        for source in [
            "name: app\ndependencies:\n- |\n  ../old",
            "name: app\ndependencies:\n- |+\n  ../old",
            "name: app\ndependencies:\n- >\n  ../old",
            "name: app\ndependencies:\n- >+\n  ../old",
            "name: app\ndependencies:\n- |+\n  ../old\n  ",
        ] {
            check(source, &["../new"]);
        }
    }

    #[test]
    fn empty_append_is_an_exact_noop_without_parsing() {
        for source in ["", "not: [valid", "# 💧\r\nname: app"] {
            assert_eq!(append(source, &[]).unwrap(), source);
        }
    }

    #[test]
    fn inspection_enforces_independent_work_budgets() {
        for (source, budget, message) in [
            (
                "name: app\nextra: [a, b, c]",
                serde_saphyr::budget! { max_events: 4 },
                "event budget",
            ),
            (
                "name: app\nextra: [[a]]",
                serde_saphyr::budget! { max_depth: 2 },
                "depth budget",
            ),
            (
                "name: app",
                serde_saphyr::budget! { max_total_scalar_bytes: 5 },
                "scalar-byte budget",
            ),
        ] {
            let error = Document::parse(source, &budget.unwrap()).err().unwrap();
            assert!(error.contains(message), "{error}");
        }
    }
}
