// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Limited `erDiagram` parse/export for Nereid's ER AST.

use std::fmt;

use crate::model::ids::ObjectId;
use crate::model::{ErAst, ErCardinality, ErEntity, ErRelationship, ErStroke};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidErParseError {
    MissingHeader,
    EmptyInput,
    UnsupportedLine { line_no: usize, line: String },
    InvalidRelationship { line_no: usize, line: String },
}

impl fmt::Display for MermaidErParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => f.write_str("expected 'erDiagram' as the first non-empty line"),
            Self::EmptyInput => f.write_str("empty er diagram input"),
            Self::UnsupportedLine { line_no, line } => {
                write!(f, "unsupported erDiagram line {line_no}: {line}")
            }
            Self::InvalidRelationship { line_no, line } => {
                write!(f, "invalid er relationship on line {line_no}: {line}")
            }
        }
    }
}

impl std::error::Error for MermaidErParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidErExportError {
    EmptyEntityName { entity_id: ObjectId },
}

impl fmt::Display for MermaidErExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEntityName { entity_id } => {
                write!(f, "entity {entity_id} has an empty name")
            }
        }
    }
}

impl std::error::Error for MermaidErExportError {}

fn sanitize_fragment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("entity");
    }
    out
}

fn entity_id_from_name(name: &str) -> ObjectId {
    ObjectId::new(format!("e:{}", sanitize_fragment(name))).expect("entity id")
}

fn ensure_entity(ast: &mut ErAst, name: &str) -> ObjectId {
    let id = entity_id_from_name(name);
    ast.entities_mut().entry(id.clone()).or_insert_with(|| ErEntity::new(name));
    id
}

/// Parse a 2-char Mermaid ER cardinality token into a 1-cell semantic kind.
pub fn fold_cardinality_token(token: &str) -> Option<ErCardinality> {
    match token {
        "||" => Some(ErCardinality::ExactlyOne),
        "|o" | "o|" => Some(ErCardinality::ZeroOrOne),
        "|{" | "}|" => Some(ErCardinality::OneOrMore),
        "}o" | "o{" => Some(ErCardinality::ZeroOrMore),
        _ => None,
    }
}

fn card_export(card: ErCardinality, left: bool) -> &'static str {
    match (card, left) {
        (ErCardinality::ExactlyOne, _) => "||",
        (ErCardinality::ZeroOrOne, true) => "|o",
        (ErCardinality::ZeroOrOne, false) => "o|",
        (ErCardinality::OneOrMore, true) => "}|",
        (ErCardinality::OneOrMore, false) => "|{",
        (ErCardinality::ZeroOrMore, true) => "}o",
        (ErCardinality::ZeroOrMore, false) => "o{",
    }
}

/// Parse limited `erDiagram` subset.
pub fn parse_er_diagram(input: &str) -> Result<ErAst, MermaidErParseError> {
    let mut lines = input.lines().enumerate().filter(|(_, l)| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with("%%")
    });

    let Some((_, first)) = lines.next() else {
        return Err(MermaidErParseError::EmptyInput);
    };
    if first.trim() != "erDiagram" {
        return Err(MermaidErParseError::MissingHeader);
    }

    let mut ast = ErAst::default();
    let mut rel_seq = 0u32;

    for (idx, line) in lines {
        let line_no = idx + 1;
        let trimmed = line.trim();

        // ENTITY_A ||--o{ ENTITY_B : label
        let (body, label) = if let Some((left, right)) = trimmed.split_once(" : ") {
            (left.trim(), Some(right.trim()))
        } else if let Some((left, right)) = trimmed.split_once(':') {
            (left.trim(), Some(right.trim()))
        } else {
            (trimmed, None)
        };

        // Find identifying `--` or non-identifying `..` between two card tokens.
        let stroke_sep = if body.contains("..") {
            ".."
        } else if body.contains("--") {
            "--"
        } else {
            // bare entity name
            if body.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
                ensure_entity(&mut ast, body);
                continue;
            }
            return Err(MermaidErParseError::UnsupportedLine { line_no, line: trimmed.to_owned() });
        };

        let stroke =
            if stroke_sep == ".." { ErStroke::NonIdentifying } else { ErStroke::Identifying };

        let Some(sep_idx) = body.find(stroke_sep) else {
            return Err(MermaidErParseError::InvalidRelationship {
                line_no,
                line: trimmed.to_owned(),
            });
        };

        let left_part = body[..sep_idx].trim();
        let right_part = body[sep_idx + stroke_sep.len()..].trim();

        // left: "CUSTOMER ||"  right: "o{ ORDER"
        let (left_name, left_card_tok) = split_name_card(left_part, true).ok_or_else(|| {
            MermaidErParseError::InvalidRelationship { line_no, line: trimmed.to_owned() }
        })?;
        let (right_card_tok, right_name) = split_name_card(right_part, false).ok_or_else(|| {
            MermaidErParseError::InvalidRelationship { line_no, line: trimmed.to_owned() }
        })?;

        let from_card = fold_cardinality_token(left_card_tok).ok_or_else(|| {
            MermaidErParseError::InvalidRelationship { line_no, line: trimmed.to_owned() }
        })?;
        let to_card = fold_cardinality_token(right_card_tok).ok_or_else(|| {
            MermaidErParseError::InvalidRelationship { line_no, line: trimmed.to_owned() }
        })?;

        let from_id = ensure_entity(&mut ast, left_name);
        let to_id = ensure_entity(&mut ast, right_name);
        rel_seq = rel_seq.saturating_add(1);
        let rel_id = ObjectId::new(format!("r:{rel_seq:04}")).expect("rel id");
        let raw = format!("{left_card_tok}{stroke_sep}{right_card_tok}");
        let rel = ErRelationship::new(from_id, to_id, from_card, to_card)
            .with_stroke(stroke)
            .with_label(label.map(str::to_owned))
            .with_raw_connector(Some(raw));
        ast.relationships_mut().insert(rel_id, rel);
    }

    Ok(ast)
}

fn split_name_card(part: &str, name_first: bool) -> Option<(&str, &str)> {
    let part = part.trim();
    // Cardinality tokens are 2 chars.
    if part.len() < 3 {
        return None;
    }
    if name_first {
        // "CUSTOMER ||" or "CUSTOMER||"
        for tok in ["||", "|o", "o|", "|{", "}|", "}o", "o{"] {
            if let Some(idx) = part.rfind(tok) {
                let name = part[..idx].trim();
                if !name.is_empty() && part[idx..].trim() == tok {
                    return Some((name, tok));
                }
            }
        }
    } else {
        // "o{ ORDER" or "o{ORDER"
        for tok in ["||", "|o", "o|", "|{", "}|", "}o", "o{"] {
            if let Some(rest) = part.strip_prefix(tok) {
                let name = rest.trim();
                if !name.is_empty() {
                    return Some((tok, name));
                }
            }
        }
    }
    None
}

/// Export ER diagram to Mermaid.
pub fn export_er_diagram(ast: &ErAst) -> Result<String, MermaidErExportError> {
    let mut out = String::from("erDiagram\n");
    for entity in ast.entities().values() {
        if entity.name().is_empty() {
            let id = ast
                .entities()
                .iter()
                .find(|(_, e)| e.name().is_empty())
                .map(|(id, _)| id.clone())
                .unwrap_or_else(|| ObjectId::new("e:unknown").expect("id"));
            return Err(MermaidErExportError::EmptyEntityName { entity_id: id });
        }
    }

    // Emit bare entity names so orphans (and note sidecars) survive mmd round-trip.
    // Relationships re-ensure the same entities on parse.
    for entity in ast.entities().values() {
        out.push_str(&format!("    {}\n", entity.name()));
    }

    for rel in ast.relationships().values() {
        let from = ast.entities().get(rel.from_entity_id()).map(ErEntity::name).unwrap_or("?");
        let to = ast.entities().get(rel.to_entity_id()).map(ErEntity::name).unwrap_or("?");
        let left = card_export(rel.from_card(), true);
        let right = card_export(rel.to_card(), false);
        let sep = match rel.stroke() {
            ErStroke::Identifying => "--",
            ErStroke::NonIdentifying => "..",
        };
        if let Some(label) = rel.label() {
            out.push_str(&format!("    {from} {left}{sep}{right} {to} : {label}\n"));
        } else {
            out.push_str(&format!("    {from} {left}{sep}{right} {to}\n"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_screenshot_like_er() {
        let input = r#"
erDiagram
CUSTOMER ||--o{ ORDER : places
ORDER ||--|{ LINE-ITEM : contains
CUSTOMER }|..|{ DELIVERY-ADDRESS : uses
"#;
        let ast = parse_er_diagram(input).expect("parse");
        assert_eq!(ast.entities().len(), 4);
        assert_eq!(ast.relationships().len(), 3);
        let places = ast.relationships().values().find(|r| r.label() == Some("places")).unwrap();
        assert_eq!(places.from_card(), ErCardinality::ExactlyOne);
        assert_eq!(places.to_card(), ErCardinality::ZeroOrMore);
    }

    #[test]
    fn fold_cardinality_is_exhaustive_for_supported_set() {
        for tok in ["||", "|o", "o|", "|{", "}|", "}o", "o{"] {
            assert!(fold_cardinality_token(tok).is_some(), "{tok}");
        }
    }

    #[test]
    fn export_roundtrip() {
        let input = "erDiagram\nA ||--o{ B : r\n";
        let a1 = parse_er_diagram(input).expect("p1");
        let out = export_er_diagram(&a1).expect("export");
        let a2 = parse_er_diagram(&out).expect("p2");
        assert_eq!(a1.entities().len(), a2.entities().len());
        assert_eq!(a1.relationships().len(), a2.relationships().len());
    }
}
