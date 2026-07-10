// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Limited `classDiagram` parse/export for Nereid's class AST.

use std::fmt;

use crate::model::ids::ObjectId;
use crate::model::{ClassAst, ClassNode, ClassRelation, ClassRelationKind};

fn sanitize_object_id_fragment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("class");
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidClassParseError {
    MissingHeader,
    EmptyInput,
    UnsupportedLine { line_no: usize, line: String },
    InvalidRelation { line_no: usize, line: String },
    EmptyClassName { line_no: usize },
}

impl fmt::Display for MermaidClassParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => {
                f.write_str("expected 'classDiagram' as the first non-empty line")
            }
            Self::EmptyInput => f.write_str("empty class diagram input"),
            Self::UnsupportedLine { line_no, line } => {
                write!(f, "unsupported classDiagram line {line_no}: {line}")
            }
            Self::InvalidRelation { line_no, line } => {
                write!(f, "invalid class relation on line {line_no}: {line}")
            }
            Self::EmptyClassName { line_no } => {
                write!(f, "empty class name on line {line_no}")
            }
        }
    }
}

impl std::error::Error for MermaidClassParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidClassExportError {
    EmptyClassName { class_id: ObjectId },
}

impl fmt::Display for MermaidClassExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyClassName { class_id } => {
                write!(f, "class {class_id} has an empty name")
            }
        }
    }
}

impl std::error::Error for MermaidClassExportError {}

fn class_id_from_name(name: &str) -> ObjectId {
    let frag = sanitize_object_id_fragment(name);
    ObjectId::new(format!("c:{frag}")).expect("class object id")
}

fn ensure_class(ast: &mut ClassAst, name: &str) -> ObjectId {
    let id = class_id_from_name(name);
    ast.classes_mut().entry(id.clone()).or_insert_with(|| ClassNode::new(name));
    id
}

/// Relation tokens ordered longest-first for greedy match.
/// Endpoints are always stored left→right as written; caps come from the raw token on lower.
const RELATION_TOKENS: &[(&str, ClassRelationKind)] = &[
    ("<-->", ClassRelationKind::Link),
    ("<|--", ClassRelationKind::Inheritance),
    ("--|>", ClassRelationKind::Inheritance),
    ("..|>", ClassRelationKind::Realization),
    ("<|..", ClassRelationKind::Realization),
    ("*--", ClassRelationKind::Composition),
    ("--*", ClassRelationKind::Composition),
    ("o--", ClassRelationKind::Aggregation),
    ("--o", ClassRelationKind::Aggregation),
    ("-->", ClassRelationKind::Association),
    ("<--", ClassRelationKind::Association),
    ("..>", ClassRelationKind::Dependency),
    ("<..", ClassRelationKind::Dependency),
    ("..", ClassRelationKind::Dependency),
    ("--", ClassRelationKind::Link),
];

fn split_relation_line(line: &str) -> Option<(&str, &str, &str, Option<&str>)> {
    let trimmed = line.trim();
    // Optional label after " : "
    let (body, label) = if let Some((left, right)) = trimmed.split_once(" : ") {
        (left.trim(), Some(right.trim()))
    } else if let Some((left, right)) = trimmed.split_once(':') {
        // "A --> B: label" without spaces around colon
        let left = left.trim();
        let right = right.trim();
        if right.is_empty() {
            (trimmed, None)
        } else {
            // Could be member line "Class : member" — only treat as relation if a token is present.
            (left, Some(right))
        }
    } else {
        (trimmed, None)
    };

    for (token, _kind) in RELATION_TOKENS {
        if let Some(idx) = body.find(token) {
            let left = body[..idx].trim();
            let right = body[idx + token.len()..].trim();
            if left.is_empty() || right.is_empty() {
                return None;
            }
            return Some((left, token, right, label));
        }
    }
    None
}

fn is_member_line(line: &str) -> Option<(&str, &str)> {
    // "ClassName : member" without a relation token.
    if split_relation_line(line).is_some() {
        // Has a relation token — not a member line (even if label present).
        // Exception: "Class : member" has ":" but no token; split_relation returns None.
    }
    let trimmed = line.trim();
    if let Some((left, right)) = trimmed.split_once(':') {
        let name = left.trim();
        let member = right.trim();
        if name.is_empty() || member.is_empty() {
            return None;
        }
        if RELATION_TOKENS.iter().any(|(t, _)| trimmed.contains(t)) {
            return None;
        }
        return Some((name, member));
    }
    None
}

fn classify_member(member: &str) -> bool {
    // Heuristic: methods contain '(' ; attributes otherwise.
    member.contains('(')
}

/// Parse a limited `classDiagram` Mermaid subset.
pub fn parse_class_diagram(input: &str) -> Result<ClassAst, MermaidClassParseError> {
    let mut lines = input.lines().enumerate().filter(|(_, l)| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with("%%")
    });

    let Some((_, first)) = lines.next() else {
        return Err(MermaidClassParseError::EmptyInput);
    };
    if first.trim() != "classDiagram" {
        return Err(MermaidClassParseError::MissingHeader);
    }

    let mut ast = ClassAst::default();
    let mut relation_seq = 0u32;

    for (idx, line) in lines {
        let line_no = idx + 1;
        let trimmed = line.trim();

        if let Some((name, member)) = is_member_line(trimmed) {
            if name.is_empty() {
                return Err(MermaidClassParseError::EmptyClassName { line_no });
            }
            let id = ensure_class(&mut ast, name);
            let class = ast.classes_mut().get_mut(&id).expect("just inserted");
            if classify_member(member) {
                class.methods_mut().push(member.to_owned());
            } else {
                class.attributes_mut().push(member.to_owned());
            }
            continue;
        }

        if let Some((left, token, right, label)) = split_relation_line(trimmed) {
            let Some((_, kind)) = RELATION_TOKENS.iter().find(|(t, _)| *t == token) else {
                return Err(MermaidClassParseError::InvalidRelation {
                    line_no,
                    line: trimmed.to_owned(),
                });
            };

            let from_id = ensure_class(&mut ast, left);
            let to_id = ensure_class(&mut ast, right);
            relation_seq = relation_seq.saturating_add(1);
            let rel_id = ObjectId::new(format!("r:{relation_seq:04}")).expect("relation id");
            let relation = ClassRelation::new(from_id, to_id, *kind)
                .with_label(label.map(str::to_owned))
                .with_raw_connector(Some((*token).to_owned()));
            ast.relations_mut().insert(rel_id, relation);
            continue;
        }

        // Bare class name line.
        if trimmed.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
            ensure_class(&mut ast, trimmed);
            continue;
        }

        return Err(MermaidClassParseError::UnsupportedLine { line_no, line: trimmed.to_owned() });
    }

    Ok(ast)
}

fn default_connector(kind: ClassRelationKind) -> &'static str {
    match kind {
        ClassRelationKind::Inheritance => "--|>",
        ClassRelationKind::Composition => "*--",
        ClassRelationKind::Aggregation => "o--",
        ClassRelationKind::Association => "-->",
        ClassRelationKind::Dependency => "..>",
        ClassRelationKind::Realization => "..|>",
        ClassRelationKind::Link => "--",
    }
}

/// Export a class diagram to canonical Mermaid `.mmd`.
pub fn export_class_diagram(ast: &ClassAst) -> Result<String, MermaidClassExportError> {
    let mut out = String::from("classDiagram\n");

    // Emit members first for stable readability.
    for (class_id, class) in ast.classes() {
        if class.name().is_empty() {
            return Err(MermaidClassExportError::EmptyClassName { class_id: class_id.clone() });
        }
        for attr in class.attributes() {
            out.push_str(&format!("    {} : {}\n", class.name(), attr));
        }
        for method in class.methods() {
            out.push_str(&format!("    {} : {}\n", class.name(), method));
        }
        if class.attributes().is_empty() && class.methods().is_empty() {
            out.push_str(&format!("    {}\n", class.name()));
        }
    }

    for relation in ast.relations().values() {
        let from = ast.classes().get(relation.from_class_id()).map(ClassNode::name).unwrap_or("?");
        let to = ast.classes().get(relation.to_class_id()).map(ClassNode::name).unwrap_or("?");
        let token = relation.raw_connector().unwrap_or_else(|| default_connector(relation.kind()));
        // Export in from→to order with raw token as stored; if reverse tokens were normalized
        // at parse, raw_connector preserves original orientation when present.
        if let Some(label) = relation.label() {
            out.push_str(&format!("    {from} {token} {to} : {label}\n"));
        } else {
            out.push_str(&format!("    {from} {token} {to}\n"));
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_screenshot_like_class_diagram() {
        let input = r#"
classDiagram
Class01 <|-- AveryLongClass : Cool
Class03 *-- Class04
Class05 o-- Class06
Class07 .. Class08
Class09 --> C2 : Where am i?
Class09 --* C3
Class09 --|> Class07
Class07 : equals()
Class07 : Object[] elementData
Class01 : size()
Class01 : int chimp
Class01 : int gorilla
Class08 <--> C2 : Cool label
"#;
        let ast = parse_class_diagram(input).expect("parse");
        assert!(ast.classes().len() >= 8);
        let c01 = class_id_from_name("Class01");
        let node = ast.classes().get(&c01).expect("Class01");
        assert!(node.methods().iter().any(|m| m == "size()"));
        assert!(node.attributes().iter().any(|a| a == "int chimp"));
        assert!(!ast.relations().is_empty());
    }

    #[test]
    fn export_roundtrip_preserves_classes_and_relations() {
        let input = "classDiagram\nA <|-- B : cool\nA : foo()\nA : x\n";
        let ast1 = parse_class_diagram(input).expect("parse1");
        let out = export_class_diagram(&ast1).expect("export");
        let ast2 = parse_class_diagram(&out).expect("parse2");
        assert_eq!(ast1.classes().len(), ast2.classes().len());
        assert_eq!(ast1.relations().len(), ast2.relations().len());
    }

    #[test]
    fn rejects_missing_header() {
        let err = parse_class_diagram("A --> B\n").unwrap_err();
        assert_eq!(err, MermaidClassParseError::MissingHeader);
    }
}
