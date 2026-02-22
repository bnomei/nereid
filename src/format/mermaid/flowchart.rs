// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::fmt;

use super::ident::validate_mermaid_ident;
pub use super::ident::MermaidIdentError;

use crate::model::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
use crate::model::ids::ObjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidFlowchartParseError {
    MissingHeader,
    InvalidDirection {
        line_no: usize,
        direction: String,
    },
    InvalidLinkStyleIndex {
        line_no: usize,
        index: usize,
        max_index: usize,
    },
    UnsupportedSyntax {
        line_no: usize,
        line: String,
    },
    InvalidNodeId {
        line_no: usize,
        name: String,
        reason: MermaidIdentError,
    },
    InvalidNodeLabelSyntax {
        line_no: usize,
        token: String,
    },
    EmptyNodeLabel {
        line_no: usize,
        token: String,
    },
    EmptyEdgeLabel {
        line_no: usize,
        line: String,
    },
    ConflictingNodeLabel {
        line_no: usize,
        mermaid_id: String,
        existing_label: String,
        new_label: String,
    },
    ConflictingNodeShape {
        line_no: usize,
        mermaid_id: String,
        existing_shape: String,
        new_shape: String,
    },
}

impl fmt::Display for MermaidFlowchartParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => f.write_str("expected 'flowchart' as the first non-empty line"),
            Self::InvalidDirection { line_no, direction } => write!(
                f,
                "invalid flowchart direction on line {line_no}: {direction} (expected TD/TB/LR/RL/BT)"
            ),
            Self::InvalidLinkStyleIndex {
                line_no,
                index,
                max_index,
            } => write!(
                f,
                "invalid flowchart linkStyle on line {line_no}: index {index} is out of bounds (max {max_index})"
            ),
            Self::UnsupportedSyntax { line_no, line } => {
                write!(f, "unsupported Mermaid syntax on line {line_no}: {line}")
            }
            Self::InvalidNodeId {
                line_no,
                name,
                reason,
            } => write!(f, "invalid node id on line {line_no}: {name} ({reason})"),
            Self::InvalidNodeLabelSyntax { line_no, token } => write!(
                f,
                "invalid node label syntax on line {line_no}: {token} (expected '<id>[<label>]', '<id>(<label>)', or '<id>{{<label>}}')"
            ),
            Self::EmptyNodeLabel { line_no, token } => {
                write!(f, "empty node label on line {line_no}: {token}")
            }
            Self::EmptyEdgeLabel { line_no, line } => {
                write!(f, "empty edge label on line {line_no}: {line}")
            }
            Self::ConflictingNodeLabel {
                line_no,
                mermaid_id,
                existing_label,
                new_label,
            } => write!(
                f,
                "conflicting label for node '{mermaid_id}' on line {line_no}: '{existing_label}' vs '{new_label}'"
            ),
            Self::ConflictingNodeShape {
                line_no,
                mermaid_id,
                existing_shape,
                new_shape,
            } => write!(
                f,
                "conflicting shape for node '{mermaid_id}' on line {line_no}: '{existing_shape}' vs '{new_shape}'"
            ),
        }
    }
}

impl std::error::Error for MermaidFlowchartParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidFlowchartExportError {
    MissingNode { node_id: ObjectId },
    InvalidNodeId { node_id: ObjectId },
    InvalidNodeLabel { node_id: ObjectId, label: String },
    InvalidNodeShape { node_id: ObjectId, shape: String },
    InvalidEdgeLabel { edge_id: ObjectId, label: String },
}

impl fmt::Display for MermaidFlowchartExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode { node_id } => {
                write!(f, "edge references missing node id: {node_id}")
            }
            Self::InvalidNodeId { node_id } => write!(
                f,
                "cannot export node id as Mermaid identifier: {node_id} (expected 'n:<ident>')"
            ),
            Self::InvalidNodeLabel { node_id, label } => write!(
                f,
                "cannot export node label for {node_id}: contains unsupported characters: {label:?}"
            ),
            Self::InvalidNodeShape { node_id, shape } => write!(
                f,
                "cannot export node {node_id}: unsupported flowchart node shape: {shape:?}"
            ),
            Self::InvalidEdgeLabel { edge_id, label } => write!(
                f,
                "cannot export edge label for {edge_id}: contains unsupported characters: {label:?}"
            ),
        }
    }
}

impl std::error::Error for MermaidFlowchartExportError {}

fn node_id_from_mermaid_id(name: &str) -> Result<ObjectId, MermaidIdentError> {
    validate_mermaid_ident(name)?;
    // Stable and human-friendly by default; long-term stability is carried in `.meta.json` sidecars.
    ObjectId::new(format!("n:{name}")).map_err(|_| MermaidIdentError::ContainsSlash)
}

fn edge_id_from_index(index: usize) -> ObjectId {
    ObjectId::new(format!("e:{index:04}")).expect("valid edge id")
}

fn is_edge_op_start_char(ch: char) -> bool {
    matches!(ch, '<' | '-' | '=' | '.')
}

fn is_edge_op_char(ch: char) -> bool {
    matches!(ch, '<' | '>' | '-' | '=' | '.' | 'o' | 'x')
}

fn is_probable_edge_operator(op: &str) -> bool {
    let mut stroke_len = 0usize;
    for ch in op.chars() {
        if matches!(ch, '-' | '=' | '.') {
            stroke_len += 1;
        }
    }
    stroke_len >= 2
}

fn split_once_edge_operator(line: &str) -> Option<(&str, &str, &str)> {
    let mut in_label: Option<char> = None;
    let mut op_start: Option<usize> = None;

    for (idx, ch) in line.char_indices() {
        if let Some(close) = in_label {
            if ch == close {
                in_label = None;
            }
            continue;
        }

        match ch {
            '[' => in_label = Some(']'),
            '(' => in_label = Some(')'),
            '{' => in_label = Some('}'),
            _ => {}
        }

        if in_label.is_some() {
            continue;
        }

        if op_start.is_none() && is_edge_op_start_char(ch) {
            op_start = Some(idx);
            break;
        }
    }

    let start = op_start?;
    let mut end = line.len();
    for (idx, ch) in line[start..].char_indices() {
        if !is_edge_op_char(ch) {
            end = start + idx;
            break;
        }
    }

    let lhs = &line[..start];
    let op = &line[start..end];
    let rhs = &line[end..];
    if lhs.trim().is_empty() || !is_probable_edge_operator(op) {
        return None;
    }

    Some((lhs, op, rhs))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeDirection {
    Forward,
    Reverse,
}

fn edge_direction(op: &str) -> EdgeDirection {
    let has_left = op.contains('<');
    let has_right = op.contains('>');
    if has_left && !has_right {
        EdgeDirection::Reverse
    } else {
        EdgeDirection::Forward
    }
}

fn normalize_edge_operator(op: &str, direction: EdgeDirection) -> String {
    let op = op.trim();
    match direction {
        EdgeDirection::Forward => op.to_owned(),
        EdgeDirection::Reverse => {
            let mut normalized = String::with_capacity(op.len().saturating_add(1));
            for ch in op.chars() {
                if ch != '<' {
                    normalized.push(ch);
                }
            }

            match normalized.chars().last() {
                Some('o' | 'x') => {
                    let decoration = normalized.pop().expect("non-empty after last()");
                    normalized.push('>');
                    normalized.push(decoration);
                }
                _ => normalized.push('>'),
            }

            normalized
        }
    }
}

fn parse_link_style_statement(
    trimmed: &str,
    line_no: usize,
) -> Result<(Option<Vec<usize>>, String), MermaidFlowchartParseError> {
    let rest = trimmed
        .strip_prefix("linkStyle")
        .ok_or_else(|| MermaidFlowchartParseError::UnsupportedSyntax {
            line_no,
            line: trimmed.to_owned(),
        })?
        .trim_start();

    let rest = rest.trim();
    let mut split_idx: Option<usize> = None;
    for (idx, ch) in rest.char_indices() {
        if ch.is_whitespace() {
            split_idx = Some(idx);
            break;
        }
    }
    let split_idx = split_idx.ok_or_else(|| MermaidFlowchartParseError::UnsupportedSyntax {
        line_no,
        line: trimmed.to_owned(),
    })?;
    let targets_raw = rest[..split_idx].trim();
    let style_raw = rest[split_idx..].trim();

    let style = style_raw.trim();
    if style.is_empty() {
        return Err(MermaidFlowchartParseError::UnsupportedSyntax {
            line_no,
            line: trimmed.to_owned(),
        });
    }

    if targets_raw == "default" {
        return Ok((None, style.to_owned()));
    }

    let mut indices = Vec::<usize>::new();
    for raw in targets_raw.split(',') {
        let value = raw.trim();
        if value.is_empty() {
            return Err(MermaidFlowchartParseError::UnsupportedSyntax {
                line_no,
                line: trimmed.to_owned(),
            });
        }
        let index: usize = value.parse().map_err(|_| {
            MermaidFlowchartParseError::UnsupportedSyntax { line_no, line: trimmed.to_owned() }
        })?;
        indices.push(index);
    }

    if indices.is_empty() {
        return Err(MermaidFlowchartParseError::UnsupportedSyntax {
            line_no,
            line: trimmed.to_owned(),
        });
    }

    Ok((Some(indices), style.to_owned()))
}

fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("%%")
}

fn is_ignorable_line(trimmed: &str) -> bool {
    trimmed.starts_with("subgraph ")
        || trimmed == "end"
        || trimmed.starts_with("style ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("classDef ")
        || trimmed.starts_with("click ")
        || trimmed.starts_with("link ")
        || trimmed.starts_with("links ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeShape {
    Rect,
    Round,
    Diamond,
}

impl NodeShape {
    fn model_shape(&self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Round => "round",
            Self::Diamond => "diamond",
        }
    }

    fn from_model_shape(shape: &str) -> Option<Self> {
        match shape {
            "rect" => Some(Self::Rect),
            "round" => Some(Self::Round),
            "diamond" => Some(Self::Diamond),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeSpec {
    mermaid_id: String,
    label: Option<String>,
    shape: Option<NodeShape>,
}

fn parse_node_spec(token: &str, line_no: usize) -> Result<NodeSpec, MermaidFlowchartParseError> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(MermaidFlowchartParseError::UnsupportedSyntax {
            line_no,
            line: token.to_owned(),
        });
    }

    let mut open_delim: Option<(usize, char)> = None;
    for (idx, ch) in trimmed.char_indices() {
        if matches!(ch, '[' | '(' | '{') {
            open_delim = Some((idx, ch));
            break;
        }
    }

    let Some((open_idx, open_ch)) = open_delim else {
        validate_mermaid_ident(trimmed).map_err(|reason| {
            MermaidFlowchartParseError::InvalidNodeId { line_no, name: trimmed.to_owned(), reason }
        })?;
        return Ok(NodeSpec { mermaid_id: trimmed.to_owned(), label: None, shape: None });
    };

    let (close_ch, shape) = match open_ch {
        '[' => (']', NodeShape::Rect),
        '(' => (')', NodeShape::Round),
        '{' => ('}', NodeShape::Diamond),
        _ => {
            return Err(MermaidFlowchartParseError::UnsupportedSyntax {
                line_no,
                line: trimmed.to_owned(),
            })
        }
    };

    let id_raw = trimmed[..open_idx].trim();
    validate_mermaid_ident(id_raw).map_err(|reason| MermaidFlowchartParseError::InvalidNodeId {
        line_no,
        name: id_raw.to_owned(),
        reason,
    })?;

    let label_raw_with_close = &trimmed[open_idx + open_ch.len_utf8()..];
    if !label_raw_with_close.ends_with(close_ch) {
        return Err(MermaidFlowchartParseError::InvalidNodeLabelSyntax {
            line_no,
            token: trimmed.to_owned(),
        });
    }

    let label_raw = &label_raw_with_close[..label_raw_with_close.len() - close_ch.len_utf8()];
    let label = label_raw.trim();
    if label.is_empty() {
        return Err(MermaidFlowchartParseError::EmptyNodeLabel {
            line_no,
            token: trimmed.to_owned(),
        });
    }

    Ok(NodeSpec {
        mermaid_id: id_raw.to_owned(),
        label: Some(label.to_owned()),
        shape: Some(shape),
    })
}

fn ensure_node(
    ast: &mut FlowchartAst,
    spec: NodeSpec,
    line_no: usize,
) -> Result<ObjectId, MermaidFlowchartParseError> {
    let NodeSpec { mermaid_id, label, shape } = spec;

    let node_id = node_id_from_mermaid_id(&mermaid_id).map_err(|reason| {
        MermaidFlowchartParseError::InvalidNodeId { line_no, name: mermaid_id.clone(), reason }
    })?;

    let desired_label = label.as_deref().unwrap_or(&mermaid_id).to_owned();
    let desired_shape =
        shape.as_ref().map(|shape| shape.model_shape()).unwrap_or(NodeShape::Rect.model_shape());

    let Some(existing) = ast.nodes().get(&node_id) else {
        ast.nodes_mut().insert(
            node_id.clone(),
            FlowNode::new_with(desired_label, desired_shape, Some(mermaid_id)),
        );
        return Ok(node_id);
    };

    let existing_label = existing.label().to_owned();
    let existing_shape = existing.shape().to_owned();
    let existing_mermaid_id = existing.mermaid_id().map(str::to_owned);

    let mut new_label = existing_label.clone();
    if let Some(explicit_label) = label {
        if existing_label == explicit_label {
            // ok
        } else if existing_label == mermaid_id {
            // implicit (default) label, upgrade to explicit label.
            new_label = explicit_label;
        } else {
            return Err(MermaidFlowchartParseError::ConflictingNodeLabel {
                line_no,
                mermaid_id: mermaid_id.clone(),
                existing_label,
                new_label: explicit_label,
            });
        }
    }

    let mut new_shape = existing_shape.clone();
    if let Some(explicit_shape) = shape {
        let explicit_shape = explicit_shape.model_shape();
        if existing_shape == explicit_shape {
            // ok
        } else if existing_shape == NodeShape::Rect.model_shape() {
            // upgrade from default rect shape to explicit shape.
            new_shape = explicit_shape.to_owned();
        } else {
            return Err(MermaidFlowchartParseError::ConflictingNodeShape {
                line_no,
                mermaid_id: mermaid_id.clone(),
                existing_shape,
                new_shape: explicit_shape.to_owned(),
            });
        }
    }

    let new_mermaid_id = Some(mermaid_id);
    if new_label != existing_label
        || new_shape != existing_shape
        || existing_mermaid_id != new_mermaid_id
    {
        ast.nodes_mut()
            .insert(node_id.clone(), FlowNode::new_with(new_label, new_shape, new_mermaid_id));
    }

    Ok(node_id)
}

/// Parse a deliberately limited modern Mermaid `flowchart` subset.
///
/// Supported:
/// - `flowchart`/`graph` header with optional direction (`TD`, `TB`, `LR`, `RL`, `BT`) (ignored)
/// - comment lines starting with `%%`
/// - node declarations: `<id>`, `<id>[<label>]`, `<id>(<label>)`, `<id>{<label>}`
/// - edges:
///   - supports a variety of Mermaid flowchart edge operators (normalized to `-->` internally)
///   - optional edge labels: `<lhs> -->|<label>| <rhs>` or `<lhs> -- <label> --> <rhs>`
///   - chain edges: `<a> --> <b> --> <c>`
/// - `linkStyle` statements are accepted and preserved on export (rendering currently ignores them)
///
/// Unsupported Mermaid syntax is rejected with an actionable error.
pub fn parse_flowchart(input: &str) -> Result<FlowchartAst, MermaidFlowchartParseError> {
    let mut ast = FlowchartAst::default();
    let mut saw_header = false;
    let mut edge_index = 0usize;
    let mut parsed_edges: Vec<ObjectId> = Vec::new();
    let mut pending_link_styles: Vec<(usize, Option<Vec<usize>>, String)> = Vec::new();

    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || is_comment_line(trimmed) {
            continue;
        }

        if !saw_header {
            let mut parts = trimmed.split_whitespace();
            let Some(keyword) = parts.next() else {
                continue;
            };

            if keyword != "flowchart" && keyword != "graph" {
                return Err(MermaidFlowchartParseError::MissingHeader);
            }

            if let Some(direction) = parts.next() {
                match direction {
                    "TD" | "TB" | "LR" | "RL" | "BT" => {}
                    _ => {
                        return Err(MermaidFlowchartParseError::InvalidDirection {
                            line_no,
                            direction: direction.to_owned(),
                        });
                    }
                }
                if parts.next().is_some() {
                    return Err(MermaidFlowchartParseError::UnsupportedSyntax {
                        line_no,
                        line: trimmed.to_owned(),
                    });
                }
            }

            saw_header = true;
            continue;
        }

        if trimmed.split_whitespace().next() == Some("linkStyle") {
            let (targets, style) = parse_link_style_statement(trimmed, line_no)?;
            pending_link_styles.push((line_no, targets, style));
            continue;
        }

        if is_ignorable_line(trimmed) {
            continue;
        }

        // Inline label syntax: `<lhs> -- <label> <op> <rhs>`
        if let Some((lhs_raw, op1, rest1)) = split_once_edge_operator(trimmed) {
            if op1 == "--" || op1 == "==" {
                let rest1 = rest1.trim_start();
                if let Some((label_raw, op2, rhs_raw)) = split_once_edge_operator(rest1) {
                    let label = label_raw.trim();
                    if !label.is_empty() {
                        let lhs_spec = parse_node_spec(lhs_raw, line_no)?;
                        let rhs_spec = parse_node_spec(rhs_raw, line_no)?;

                        let direction = edge_direction(op2);
                        let connector = normalize_edge_operator(op2, direction);
                        let (from_spec, to_spec) = match direction {
                            EdgeDirection::Forward => (lhs_spec, rhs_spec),
                            EdgeDirection::Reverse => (rhs_spec, lhs_spec),
                        };

                        let from_node_id = ensure_node(&mut ast, from_spec, line_no)?;
                        let to_node_id = ensure_node(&mut ast, to_spec, line_no)?;

                        edge_index += 1;
                        let edge_id = edge_id_from_index(edge_index);
                        parsed_edges.push(edge_id.clone());
                        let mut edge = FlowEdge::new_with(
                            from_node_id.clone(),
                            to_node_id.clone(),
                            Some(label.to_owned()),
                            None,
                        );
                        edge.set_connector((connector != "-->").then_some(connector));
                        ast.edges_mut().insert(edge_id, edge);
                        continue;
                    }
                }
            }
        }

        // Parse simple edge or edge chain.
        let Some((first_raw, first_op, tail)) = split_once_edge_operator(trimmed) else {
            let node_spec = parse_node_spec(trimmed, line_no)?;
            ensure_node(&mut ast, node_spec, line_no)?;
            continue;
        };

        let mut current_spec = parse_node_spec(first_raw, line_no)?;
        let mut op = first_op;
        let mut rest = tail;

        loop {
            let mut edge_label: Option<String> = None;
            let rhs_and_more = rest.trim_start();
            let rhs_and_more = if let Some(after) = rhs_and_more.strip_prefix('|') {
                let Some(end_idx) = after.find('|') else {
                    return Err(MermaidFlowchartParseError::UnsupportedSyntax {
                        line_no,
                        line: trimmed.to_owned(),
                    });
                };
                let label_raw = &after[..end_idx];
                let label = label_raw.trim();
                if label.is_empty() {
                    return Err(MermaidFlowchartParseError::EmptyEdgeLabel {
                        line_no,
                        line: trimmed.to_owned(),
                    });
                }
                edge_label = Some(label.to_owned());
                after[end_idx + 1..].trim_start()
            } else {
                rhs_and_more
            };

            let (rhs_raw, next_op, next_rest) = match split_once_edge_operator(rhs_and_more) {
                Some((rhs_raw, next_op, next_rest)) => (rhs_raw, Some(next_op), Some(next_rest)),
                None => (rhs_and_more, None, None),
            };
            let rhs_spec = parse_node_spec(rhs_raw, line_no)?;

            let direction = edge_direction(op);
            let connector = normalize_edge_operator(op, direction);
            let (from_spec, to_spec) = match direction {
                EdgeDirection::Forward => (current_spec.clone(), rhs_spec.clone()),
                EdgeDirection::Reverse => (rhs_spec.clone(), current_spec.clone()),
            };

            let from_node_id = ensure_node(&mut ast, from_spec, line_no)?;
            let to_node_id = ensure_node(&mut ast, to_spec, line_no)?;

            edge_index += 1;
            let edge_id = edge_id_from_index(edge_index);
            parsed_edges.push(edge_id.clone());
            let mut edge =
                FlowEdge::new_with(from_node_id.clone(), to_node_id.clone(), edge_label, None);
            edge.set_connector((connector != "-->").then_some(connector));
            ast.edges_mut().insert(edge_id, edge);

            let Some(next_op) = next_op else {
                break;
            };

            current_spec = rhs_spec;
            op = next_op;
            rest = next_rest.expect("next_rest present with next_op");
        }
    }

    if !saw_header {
        return Err(MermaidFlowchartParseError::MissingHeader);
    }

    for (line_no, targets, style) in pending_link_styles {
        match targets {
            None => {
                ast.set_default_edge_style(Some(style));
            }
            Some(indices) => {
                let max_index = parsed_edges.len().saturating_sub(1);
                for index in indices {
                    let Some(edge_id) = parsed_edges.get(index) else {
                        return Err(MermaidFlowchartParseError::InvalidLinkStyleIndex {
                            line_no,
                            index,
                            max_index,
                        });
                    };
                    if let Some(edge) = ast.edges_mut().get_mut(edge_id) {
                        edge.set_style(Some(style.clone()));
                    }
                }
            }
        }
    }

    Ok(ast)
}

fn mermaid_id_for_node<'a>(node_id: &'a ObjectId, node: &'a FlowNode) -> Option<&'a str> {
    node.mermaid_id().or_else(|| node_id.as_str().strip_prefix("n:"))
}

fn validate_export_node_label(label: &str, closing: char) -> bool {
    !label.contains(closing) && !label.contains('\n') && !label.contains('\r')
}

fn validate_export_edge_label(label: &str) -> bool {
    !label.contains('|') && !label.contains('\n') && !label.contains('\r')
}

fn validate_export_edge_operator(op: &str) -> bool {
    let op = op.trim();
    if op.is_empty()
        || op.contains('|')
        || op.contains('\n')
        || op.contains('\r')
        || op.chars().any(|ch| ch.is_whitespace())
    {
        return false;
    }
    if !op.chars().all(is_edge_op_char) {
        return false;
    }
    is_probable_edge_operator(op)
}

/// Export a `flowchart` to canonical Mermaid `.mmd`.
///
/// Export is stable/deterministic:
/// - Nodes are emitted in `ObjectId` order (typically lexical by `n:<id>`).
/// - Edges are emitted sorted by `(from_node_id, to_node_id, edge_id)`.
pub fn export_flowchart(ast: &FlowchartAst) -> Result<String, MermaidFlowchartExportError> {
    let mut out = String::new();
    out.push_str("flowchart\n");

    for (node_id, node) in ast.nodes() {
        let Some(mermaid_id) = mermaid_id_for_node(node_id, node) else {
            return Err(MermaidFlowchartExportError::InvalidNodeId { node_id: node_id.clone() });
        };
        validate_mermaid_ident(mermaid_id)
            .map_err(|_| MermaidFlowchartExportError::InvalidNodeId { node_id: node_id.clone() })?;

        let shape = NodeShape::from_model_shape(node.shape()).ok_or_else(|| {
            MermaidFlowchartExportError::InvalidNodeShape {
                node_id: node_id.clone(),
                shape: node.shape().to_owned(),
            }
        })?;

        let label = node.label();
        let closing = match shape {
            NodeShape::Rect => ']',
            NodeShape::Round => ')',
            NodeShape::Diamond => '}',
        };
        if !validate_export_node_label(label, closing) {
            return Err(MermaidFlowchartExportError::InvalidNodeLabel {
                node_id: node_id.clone(),
                label: label.to_owned(),
            });
        }

        out.push_str(mermaid_id);
        match shape {
            NodeShape::Rect => {
                if label != mermaid_id {
                    out.push('[');
                    out.push_str(label);
                    out.push(']');
                }
            }
            NodeShape::Round => {
                out.push('(');
                out.push_str(label);
                out.push(')');
            }
            NodeShape::Diamond => {
                out.push('{');
                out.push_str(label);
                out.push('}');
            }
        }
        out.push('\n');
    }

    let mut edges = ast.edges().iter().collect::<Vec<_>>();
    edges.sort_by(|(edge_id_a, edge_a), (edge_id_b, edge_b)| {
        edge_a
            .from_node_id()
            .as_str()
            .cmp(edge_b.from_node_id().as_str())
            .then_with(|| edge_a.to_node_id().as_str().cmp(edge_b.to_node_id().as_str()))
            .then_with(|| edge_id_a.as_str().cmp(edge_id_b.as_str()))
    });

    let mut styled_links = Vec::<(usize, String)>::new();
    for (edge_idx, (edge_id, edge)) in edges.into_iter().enumerate() {
        let from_node_id = edge.from_node_id();
        let to_node_id = edge.to_node_id();

        if !ast.nodes().contains_key(from_node_id) {
            return Err(MermaidFlowchartExportError::MissingNode { node_id: from_node_id.clone() });
        }
        if !ast.nodes().contains_key(to_node_id) {
            return Err(MermaidFlowchartExportError::MissingNode { node_id: to_node_id.clone() });
        }

        let from_node = ast.nodes().get(from_node_id).ok_or_else(|| {
            MermaidFlowchartExportError::MissingNode { node_id: from_node_id.clone() }
        })?;
        let to_node = ast.nodes().get(to_node_id).ok_or_else(|| {
            MermaidFlowchartExportError::MissingNode { node_id: to_node_id.clone() }
        })?;

        let from = mermaid_id_for_node(from_node_id, from_node).ok_or_else(|| {
            MermaidFlowchartExportError::InvalidNodeId { node_id: from_node_id.clone() }
        })?;
        let to = mermaid_id_for_node(to_node_id, to_node).ok_or_else(|| {
            MermaidFlowchartExportError::InvalidNodeId { node_id: to_node_id.clone() }
        })?;

        out.push_str(from);
        out.push(' ');
        let op = edge.connector().filter(|op| validate_export_edge_operator(op)).unwrap_or("-->");
        out.push_str(op);
        if let Some(label) = edge.label() {
            if !validate_export_edge_label(label) {
                return Err(MermaidFlowchartExportError::InvalidEdgeLabel {
                    edge_id: edge_id.clone(),
                    label: label.to_owned(),
                });
            }
            out.push('|');
            out.push_str(label);
            out.push('|');
        }
        out.push(' ');
        out.push_str(to);
        out.push('\n');

        if let Some(style) = edge.style().filter(|style| !style.is_empty()) {
            styled_links.push((edge_idx, style.to_owned()));
        }
    }

    if let Some(style) = ast.default_edge_style().filter(|style| !style.is_empty()) {
        out.push_str("linkStyle default ");
        out.push_str(style);
        out.push('\n');
    }
    for (edge_idx, style) in styled_links {
        out.push_str("linkStyle ");
        out.push_str(&edge_idx.to_string());
        out.push(' ');
        out.push_str(&style);
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{export_flowchart, parse_flowchart, MermaidFlowchartParseError};
    use crate::model::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
    use crate::model::ObjectId;
    use std::collections::BTreeMap;

    type FlowNodeSemanticView = BTreeMap<String, (String, String)>;
    type FlowEdgeSemanticView = BTreeMap<(String, String, Option<String>), usize>;

    fn semantic_view(ast: &FlowchartAst) -> (FlowNodeSemanticView, FlowEdgeSemanticView) {
        let nodes = ast
            .nodes()
            .iter()
            .map(|(node_id, node)| {
                let mermaid_id = node_id.as_str().strip_prefix("n:").expect("node id prefix");
                (mermaid_id.to_owned(), (node.label().to_owned(), node.shape().to_owned()))
            })
            .collect::<BTreeMap<_, _>>();

        let mut edges = BTreeMap::<(String, String, Option<String>), usize>::new();
        for edge in ast.edges().values() {
            let from =
                edge.from_node_id().as_str().strip_prefix("n:").expect("from prefix").to_owned();
            let to = edge.to_node_id().as_str().strip_prefix("n:").expect("to prefix").to_owned();
            let label = edge.label().map(str::to_owned);
            *edges.entry((from, to, label)).or_insert(0) += 1;
        }

        (nodes, edges)
    }

    fn connector_view(ast: &FlowchartAst) -> BTreeMap<(String, String), Option<String>> {
        let mut connectors = BTreeMap::<(String, String), Option<String>>::new();
        for edge in ast.edges().values() {
            let from =
                edge.from_node_id().as_str().strip_prefix("n:").expect("from prefix").to_owned();
            let to = edge.to_node_id().as_str().strip_prefix("n:").expect("to prefix").to_owned();
            connectors.insert((from, to), edge.connector().map(ToOwned::to_owned));
        }
        connectors
    }

    #[test]
    fn parses_nodes_and_edges() {
        let input = r#"
            %% comment
            flowchart TD
            A[Start]
            B[End]
            A --> B
        "#;

        let ast = parse_flowchart(input).expect("parse");
        let (nodes, edges) = semantic_view(&ast);

        assert_eq!(
            nodes,
            [
                ("A".to_owned(), ("Start".to_owned(), "rect".to_owned())),
                ("B".to_owned(), ("End".to_owned(), "rect".to_owned()))
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            edges,
            [((String::from("A"), String::from("B"), None), 1)].into_iter().collect()
        );
    }

    #[test]
    fn creates_implicit_nodes_from_edges_and_allows_label_upgrade() {
        let input = r#"
            flowchart
            A --> B
            A[Start]
        "#;
        let ast = parse_flowchart(input).expect("parse");
        let (nodes, edges) = semantic_view(&ast);

        assert_eq!(
            nodes,
            [
                ("A".to_owned(), ("Start".to_owned(), "rect".to_owned())),
                ("B".to_owned(), ("B".to_owned(), "rect".to_owned()))
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            edges,
            [((String::from("A"), String::from("B"), None), 1)].into_iter().collect()
        );
    }

    #[test]
    fn rejects_conflicting_node_labels() {
        let input = "flowchart\nA[Start]\nA[Begin]\n";
        let err = parse_flowchart(input).unwrap_err();
        assert!(matches!(err, MermaidFlowchartParseError::ConflictingNodeLabel { .. }));
    }

    #[test]
    fn parse_populates_node_mermaid_ids() {
        let ast = parse_flowchart("flowchart\nA[Start]\nA --> B\n").expect("parse");
        let node_a = ast.nodes().get(&ObjectId::new("n:A").expect("node id")).expect("node A");
        let node_b = ast.nodes().get(&ObjectId::new("n:B").expect("node id")).expect("node B");
        assert_eq!(node_a.mermaid_id(), Some("A"));
        assert_eq!(node_b.mermaid_id(), Some("B"));
    }

    #[test]
    fn semantic_roundtrip_parse_export_parse() {
        let input = r#"
            flowchart
            B[End]
            A[Start]
            A --> B
        "#;

        let ast1 = parse_flowchart(input).expect("parse 1");
        let out = export_flowchart(&ast1).expect("export");
        let ast2 = parse_flowchart(&out).expect("parse 2");

        assert_eq!(semantic_view(&ast1), semantic_view(&ast2));
    }

    #[test]
    fn parses_edge_labels_and_node_shapes() {
        let input = r#"
            flowchart
            A{Decide}
            B(Run)
            A -->|maybe| B
        "#;

        let ast = parse_flowchart(input).expect("parse");
        let (nodes, edges) = semantic_view(&ast);

        assert_eq!(
            nodes,
            [
                ("A".to_owned(), ("Decide".to_owned(), "diamond".to_owned())),
                ("B".to_owned(), ("Run".to_owned(), "round".to_owned()))
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            edges,
            [((String::from("A"), String::from("B"), Some(String::from("maybe"))), 1)]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn semantic_roundtrip_including_edge_labels_and_shapes() {
        let input = r#"
            flowchart
            %% create implicit nodes first
            A -->|maybe| B
            %% then refine shape+labels
            A{Decide}
            B(Run)
        "#;

        let ast1 = parse_flowchart(input).expect("parse 1");
        let out = export_flowchart(&ast1).expect("export");
        let ast2 = parse_flowchart(&out).expect("parse 2");

        assert_eq!(semantic_view(&ast1), semantic_view(&ast2));
    }

    #[test]
    fn accepts_graph_header_and_various_edge_operators() {
        let input = r#"
            %% upstream often uses graph
            graph LR
            A --- B
            B -.-> C
            C ==> D
            D <-- E
            E ---o F
            F ---x G
            A --> B --> C
        "#;

        let ast = parse_flowchart(input).expect("parse");
        assert_eq!(ast.nodes().len(), 7);
        assert_eq!(ast.edges().len(), 8);
    }

    #[test]
    fn parses_inline_edge_labels_and_link_styles() {
        let input = r#"
            flowchart
            A -- yes --> B
            A --> C
            linkStyle 0 stroke:#ff3,stroke-width:4px,color:red;
            linkStyle default stroke:#333;
        "#;

        let ast1 = parse_flowchart(input).expect("parse");
        assert_eq!(ast1.edges().len(), 2);
        assert_eq!(ast1.default_edge_style(), Some("stroke:#333;"));
        let styles = ast1
            .edges()
            .values()
            .filter_map(|edge| edge.style().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
        assert_eq!(styles, vec!["stroke:#ff3,stroke-width:4px,color:red;".to_owned()]);

        let out = export_flowchart(&ast1).expect("export");
        let ast2 = parse_flowchart(&out).expect("parse 2");
        assert_eq!(ast2.default_edge_style(), ast1.default_edge_style());
        let out_styles = ast2
            .edges()
            .values()
            .filter_map(|edge| edge.style().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
        assert_eq!(out_styles, styles);
    }

    #[test]
    fn preserves_non_default_edge_operators_on_export() {
        let input = r#"
            flowchart
            A --> B
            B -.-> C
            C ==> D
            D --- E
            E ---o F
            F ---x G
        "#;

        let ast1 = parse_flowchart(input).expect("parse 1");
        assert_eq!(
            connector_view(&ast1),
            [
                ((String::from("A"), String::from("B")), None),
                ((String::from("B"), String::from("C")), Some(String::from("-.->"))),
                ((String::from("C"), String::from("D")), Some(String::from("==>"))),
                ((String::from("D"), String::from("E")), Some(String::from("---"))),
                ((String::from("E"), String::from("F")), Some(String::from("---o"))),
                ((String::from("F"), String::from("G")), Some(String::from("---x"))),
            ]
            .into_iter()
            .collect()
        );

        let out = export_flowchart(&ast1).expect("export");
        let ast2 = parse_flowchart(&out).expect("parse 2");
        assert_eq!(connector_view(&ast2), connector_view(&ast1));
    }

    #[test]
    fn export_prefers_flow_node_mermaid_id_over_node_id_suffix() {
        let mut ast = FlowchartAst::default();
        let stable_authorize_id = ObjectId::new("n:authorize").expect("node id");
        let stable_end_id = ObjectId::new("n:end").expect("node id");
        let edge_id = ObjectId::new("e:0001").expect("edge id");

        let mut authorize = FlowNode::new("Authorize");
        authorize.set_mermaid_id(Some("authz"));
        let mut end = FlowNode::new("Finish");
        end.set_mermaid_id(Some("done"));

        ast.nodes_mut().insert(stable_authorize_id.clone(), authorize);
        ast.nodes_mut().insert(stable_end_id.clone(), end);
        ast.edges_mut().insert(edge_id, FlowEdge::new(stable_authorize_id, stable_end_id));

        let out = export_flowchart(&ast).expect("export");
        assert!(out.contains("authz[Authorize]"));
        assert!(out.contains("done[Finish]"));
        assert!(out.contains("authz --> done"));
    }

    #[test]
    fn rejects_missing_header() {
        let err = parse_flowchart("A --> B\n").unwrap_err();
        assert_eq!(err, MermaidFlowchartParseError::MissingHeader);
    }
}
