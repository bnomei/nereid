// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Family-level scene models shared by the coherent render pipeline.
//!
//! Domain ASTs (flow, sequence, class, er, gantt) lower into either a [`GraphModel`] or a
//! [`TrackModel`]. Layout and paint operate on these families so new diagram kinds add lowerers
//! rather than private layout/render stacks.
//!
//! Commit-1 note: layout/paint still bridge through existing domain renderers via
//! [`GraphModel::to_flowchart_ast`] / [`TrackModel::as_sequence_ast`]. Later commits remove the
//! bridge as graph compartments and track bars paint directly from the scene.

use std::collections::BTreeMap;

use crate::model::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
use crate::model::ids::ObjectId;
use crate::model::seq_ast::SequenceAst;
use crate::model::symbol::SymbolAnchor;

/// Single-cell endpoint glyph kind. Paint never emits multi-cell endings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum CapKind {
    #[default]
    None,
    Arrow,
    Circle,
    Cross,
    /// Class composition (`*--`).
    DiamondFilled,
    /// Class aggregation (`o--`) when distinct from circle.
    DiamondHollow,
    /// Generalization / inheritance.
    TriangleHollow,
    /// ER exactly-one (`||`).
    ExactlyOne,
    /// ER many / crow's foot.
    CrowFoot,
    /// ER zero-or-one (alias of circle semantically; kept explicit for lowerers).
    ZeroOrOne,
}

impl CapKind {
    /// Maps a Mermaid flowchart-style connector token to start/end caps.
    pub fn from_flow_connector(connector: Option<&str>) -> (Self, Self) {
        let op = connector.unwrap_or("-->").trim();
        if op.is_empty() {
            return (Self::None, Self::None);
        }

        let start = match op.chars().next() {
            Some('<') => Self::Arrow,
            Some('o') => Self::Circle,
            Some('x') => Self::Cross,
            _ => Self::None,
        };

        let end = if op.ends_with('o') {
            Self::Circle
        } else if op.ends_with('x') {
            Self::Cross
        } else if op.ends_with('>') {
            Self::Arrow
        } else {
            Self::None
        };

        (start, end)
    }

    /// One-cell unicode for an outward-facing cap. `None` has no glyph.
    pub fn glyph(self, outward_dx: i32, outward_dy: i32) -> Option<char> {
        let toward_dx = -outward_dx;
        let toward_dy = -outward_dy;
        match self {
            Self::None => None,
            Self::Arrow => Some(arrow_glyph(toward_dx, toward_dy)),
            Self::Circle | Self::ZeroOrOne => Some('○'),
            Self::Cross => Some('✕'),
            Self::DiamondFilled => Some('◆'),
            Self::DiamondHollow => Some('◇'),
            Self::TriangleHollow => Some(triangle_glyph(toward_dx, toward_dy)),
            Self::ExactlyOne => Some('‖'),
            Self::CrowFoot => Some(crow_glyph(toward_dx, toward_dy)),
        }
    }
}

fn arrow_glyph(toward_dx: i32, toward_dy: i32) -> char {
    if toward_dx.abs() >= toward_dy.abs() {
        if toward_dx < 0 {
            '◀'
        } else if toward_dx > 0 {
            '▶'
        } else if toward_dy < 0 {
            '▲'
        } else {
            '▼'
        }
    } else if toward_dy < 0 {
        '▲'
    } else {
        '▼'
    }
}

fn triangle_glyph(toward_dx: i32, toward_dy: i32) -> char {
    if toward_dx.abs() >= toward_dy.abs() {
        if toward_dx < 0 {
            '◁'
        } else if toward_dx > 0 {
            '▷'
        } else if toward_dy < 0 {
            '△'
        } else {
            '▽'
        }
    } else if toward_dy < 0 {
        '△'
    } else {
        '▽'
    }
}

fn crow_glyph(toward_dx: i32, toward_dy: i32) -> char {
    // Single-cell crow's foot; orientation follows the edge into the entity.
    if toward_dx.abs() >= toward_dy.abs() {
        if toward_dx < 0 {
            '⊂'
        } else if toward_dx > 0 {
            '⊃'
        } else if toward_dy < 0 {
            '∪'
        } else {
            '∩'
        }
    } else if toward_dy < 0 {
        '∪'
    } else {
        '∩'
    }
}

/// Edge stroke style for graph family edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EdgeStroke {
    #[default]
    Solid,
    Dashed,
}

impl EdgeStroke {
    /// Derive stroke from a Mermaid connector token and optional style string.
    ///
    /// Paint may still ignore stroke until graph paint consumes this field; lowerers should set it
    /// so dashed class/ER edges are not lost when that lands.
    pub fn from_flow_connector_and_style(connector: Option<&str>, style: Option<&str>) -> Self {
        if style.is_some_and(|s| s.to_ascii_lowercase().contains("dashed")) {
            return Self::Dashed;
        }
        let op = connector.unwrap_or("");
        // Mermaid dotted/dashed edge forms use `.` in the connector body (`-.->`, `-.`, etc.).
        if op.contains('.') {
            Self::Dashed
        } else {
            Self::Solid
        }
    }
}

/// One text compartment inside a graph node box (class attributes/methods, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphCompartment {
    lines: Vec<String>,
}

impl GraphCompartment {
    pub fn new(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { lines: lines.into_iter().map(Into::into).collect() }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Graph-family node (flow box, class, entity, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    mermaid_id: Option<String>,
    label: String,
    /// Flow shape token (`rect`, `circle`, …). Unicode paint may ignore it today.
    shape: String,
    note: Option<String>,
    symbol: Option<SymbolAnchor>,
    /// Extra stacked compartments below the title row. Empty for plain flow nodes.
    compartments: Vec<GraphCompartment>,
}

impl GraphNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            mermaid_id: None,
            label: label.into(),
            shape: "rect".to_owned(),
            note: None,
            symbol: None,
            compartments: Vec::new(),
        }
    }

    pub fn with_mermaid_id(mut self, mermaid_id: Option<impl Into<String>>) -> Self {
        self.mermaid_id = mermaid_id.map(Into::into);
        self
    }

    pub fn with_shape(mut self, shape: impl Into<String>) -> Self {
        self.shape = shape.into();
        self
    }

    pub fn with_note(mut self, note: Option<impl Into<String>>) -> Self {
        self.note = note.map(Into::into);
        self
    }

    pub fn with_symbol(mut self, symbol: Option<SymbolAnchor>) -> Self {
        self.symbol = symbol;
        self
    }

    pub fn with_compartments(mut self, compartments: Vec<GraphCompartment>) -> Self {
        self.compartments = compartments;
        self
    }

    pub fn mermaid_id(&self) -> Option<&str> {
        self.mermaid_id.as_deref()
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn shape(&self) -> &str {
        &self.shape
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn symbol(&self) -> Option<&SymbolAnchor> {
        self.symbol.as_ref()
    }

    pub fn compartments(&self) -> &[GraphCompartment] {
        &self.compartments
    }
}

/// Graph-family edge with resolved 1-cell caps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    from_node_id: ObjectId,
    to_node_id: ObjectId,
    label: Option<String>,
    /// Original Mermaid connector token when known (bridge/export).
    connector: Option<String>,
    style: Option<String>,
    start_cap: CapKind,
    end_cap: CapKind,
    stroke: EdgeStroke,
}

impl GraphEdge {
    pub fn new(from_node_id: ObjectId, to_node_id: ObjectId) -> Self {
        Self {
            from_node_id,
            to_node_id,
            label: None,
            connector: None,
            style: None,
            start_cap: CapKind::None,
            end_cap: CapKind::Arrow,
            stroke: EdgeStroke::Solid,
        }
    }

    pub fn with_label(mut self, label: Option<impl Into<String>>) -> Self {
        self.label = label.map(Into::into);
        self
    }

    pub fn with_connector(mut self, connector: Option<impl Into<String>>) -> Self {
        self.connector = connector.map(Into::into);
        self
    }

    pub fn with_style(mut self, style: Option<impl Into<String>>) -> Self {
        self.style = style.map(Into::into);
        self
    }

    pub fn with_caps(mut self, start_cap: CapKind, end_cap: CapKind) -> Self {
        self.start_cap = start_cap;
        self.end_cap = end_cap;
        self
    }

    pub fn with_stroke(mut self, stroke: EdgeStroke) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn from_node_id(&self) -> &ObjectId {
        &self.from_node_id
    }

    pub fn to_node_id(&self) -> &ObjectId {
        &self.to_node_id
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn connector(&self) -> Option<&str> {
        self.connector.as_deref()
    }

    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }

    pub fn start_cap(&self) -> CapKind {
        self.start_cap
    }

    pub fn end_cap(&self) -> CapKind {
        self.end_cap
    }

    pub fn stroke(&self) -> EdgeStroke {
        self.stroke
    }
}

/// Graph family scene: nodes + directed edges (flow / class / er).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphModel {
    nodes: BTreeMap<ObjectId, GraphNode>,
    edges: BTreeMap<ObjectId, GraphEdge>,
}

impl GraphModel {
    pub fn nodes(&self) -> &BTreeMap<ObjectId, GraphNode> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut BTreeMap<ObjectId, GraphNode> {
        &mut self.nodes
    }

    pub fn edges(&self) -> &BTreeMap<ObjectId, GraphEdge> {
        &self.edges
    }

    pub fn edges_mut(&mut self) -> &mut BTreeMap<ObjectId, GraphEdge> {
        &mut self.edges
    }

    /// Bridge to the existing flowchart layout/render stack (temporary).
    ///
    /// Restores domain flow fields so layout/paint stay lossless while paint still reads
    /// connector tokens from the bridged AST. [`GraphEdge`] caps/stroke are resolved for later
    /// direct graph paint (connector remains authoritative in flowchart helpers today).
    pub fn to_flowchart_ast(&self) -> FlowchartAst {
        let mut ast = FlowchartAst::default();
        for (id, node) in &self.nodes {
            let mut flow_node =
                FlowNode::new_with(node.label.clone(), node.shape.clone(), node.mermaid_id.clone());
            flow_node.set_note(node.note.clone());
            flow_node.set_symbol(node.symbol.clone());
            ast.nodes_mut().insert(id.clone(), flow_node);
        }
        for (id, edge) in &self.edges {
            let mut flow_edge = FlowEdge::new(edge.from_node_id.clone(), edge.to_node_id.clone());
            flow_edge.set_label(edge.label.clone());
            flow_edge.set_connector(edge.connector.clone());
            flow_edge.set_style(edge.style.clone());
            ast.edges_mut().insert(id.clone(), flow_edge);
        }
        ast
    }
}

/// How a track span is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TrackSpanStyle {
    /// Sequence message arrow.
    #[default]
    Arrow,
    /// Gantt (or similar) filled duration bar.
    Bar,
}

/// Track-family scene: multi-column spans (sequence / gantt).
///
/// Commit-1 carries the full sequence AST so block frames and layout stay lossless. Gantt will
/// populate the same family with bar spans without lifelines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackModel {
    sequence: SequenceAst,
    default_span_style: TrackSpanStyle,
}

impl TrackModel {
    pub fn from_sequence(sequence: SequenceAst) -> Self {
        Self { sequence, default_span_style: TrackSpanStyle::Arrow }
    }

    pub fn as_sequence_ast(&self) -> &SequenceAst {
        &self.sequence
    }

    pub fn into_sequence_ast(self) -> SequenceAst {
        self.sequence
    }

    pub fn default_span_style(&self) -> TrackSpanStyle {
        self.default_span_style
    }

    pub fn with_default_span_style(mut self, style: TrackSpanStyle) -> Self {
        self.default_span_style = style;
        self
    }
}

/// Kind-erased scene after lowering a domain AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderScene {
    Graph(GraphModel),
    Track(TrackModel),
}

impl RenderScene {
    pub fn family_name(&self) -> &'static str {
        match self {
            Self::Graph(_) => "graph",
            Self::Track(_) => "track",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::flow_ast::{FlowEdge, FlowNode};
    use crate::model::ids::ObjectId;

    fn oid(s: &str) -> ObjectId {
        ObjectId::new(s).expect("object id")
    }

    #[test]
    fn flow_connector_caps_match_legacy_rules() {
        assert_eq!(CapKind::from_flow_connector(Some("-->")), (CapKind::None, CapKind::Arrow));
        assert_eq!(CapKind::from_flow_connector(Some("<-->")), (CapKind::Arrow, CapKind::Arrow));
        assert_eq!(CapKind::from_flow_connector(Some("---o")), (CapKind::None, CapKind::Circle));
        assert_eq!(CapKind::from_flow_connector(Some("o--x")), (CapKind::Circle, CapKind::Cross));
        assert_eq!(CapKind::from_flow_connector(None), (CapKind::None, CapKind::Arrow));
    }

    #[test]
    fn every_cap_kind_emits_exactly_one_unicode_scalar_when_present() {
        for kind in [
            CapKind::None,
            CapKind::Arrow,
            CapKind::Circle,
            CapKind::Cross,
            CapKind::DiamondFilled,
            CapKind::DiamondHollow,
            CapKind::TriangleHollow,
            CapKind::ExactlyOne,
            CapKind::CrowFoot,
            CapKind::ZeroOrOne,
        ] {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let g = kind.glyph(dx, dy);
                if kind == CapKind::None {
                    assert!(g.is_none());
                } else {
                    let ch = g.expect("cap paints one cell");
                    // Cap is a single Rust char (one Unicode scalar); never multi-cell strings.
                    assert!(ch.len_utf8() > 0);
                }
            }
        }
    }

    #[test]
    fn arrow_circle_cross_glyphs_match_legacy_endpoint_orientations() {
        // `glyph` takes outward direction from the node along the edge; the arrow points
        // toward the node (opposite outward), matching flowchart endpoint_cap_char.
        assert_eq!(CapKind::Arrow.glyph(1, 0), Some('◀'));
        assert_eq!(CapKind::Arrow.glyph(-1, 0), Some('▶'));
        assert_eq!(CapKind::Arrow.glyph(0, 1), Some('▲'));
        assert_eq!(CapKind::Arrow.glyph(0, -1), Some('▼'));
        assert_eq!(CapKind::Circle.glyph(1, 0), Some('○'));
        assert_eq!(CapKind::Cross.glyph(1, 0), Some('✕'));
    }

    #[test]
    fn edge_stroke_detects_dotted_connectors_and_dashed_style() {
        assert_eq!(EdgeStroke::from_flow_connector_and_style(Some("-->"), None), EdgeStroke::Solid);
        assert_eq!(
            EdgeStroke::from_flow_connector_and_style(Some("-.->"), None),
            EdgeStroke::Dashed
        );
        assert_eq!(
            EdgeStroke::from_flow_connector_and_style(Some("-->"), Some("stroke-dasharray: 5")),
            EdgeStroke::Solid
        );
        assert_eq!(
            EdgeStroke::from_flow_connector_and_style(Some("-->"), Some("dashed")),
            EdgeStroke::Dashed
        );
    }

    #[test]
    fn graph_model_bridge_roundtrip_preserves_flow_fields() {
        let mut ast = FlowchartAst::default();
        let a = oid("n:a");
        let b = oid("n:b");
        let mut na = FlowNode::new_with("A", "circle", Some("A".to_owned()));
        na.set_note(Some("note-a".to_owned()));
        ast.nodes_mut().insert(a.clone(), na);
        ast.nodes_mut().insert(b.clone(), FlowNode::new("B"));
        let mut edge = FlowEdge::new(a.clone(), b.clone());
        edge.set_label(Some("go".to_owned()));
        edge.set_connector(Some("<-->".to_owned()));
        edge.set_style(Some("dashed".to_owned()));
        ast.edges_mut().insert(oid("e:1"), edge);

        let model = crate::render::lower::lower_flowchart(&ast);
        assert_eq!(model.nodes().get(&a).unwrap().shape(), "circle");
        assert_eq!(model.edges().get(&oid("e:1")).unwrap().stroke(), EdgeStroke::Dashed);
        assert_eq!(model.edges().get(&oid("e:1")).unwrap().start_cap(), CapKind::Arrow);
        assert_eq!(model.edges().get(&oid("e:1")).unwrap().end_cap(), CapKind::Arrow);

        let back = model.to_flowchart_ast();
        assert_eq!(back.nodes().len(), 2);
        assert_eq!(back.nodes().get(&a).unwrap().label(), "A");
        assert_eq!(back.nodes().get(&a).unwrap().shape(), "circle");
        assert_eq!(back.nodes().get(&a).unwrap().mermaid_id(), Some("A"));
        assert_eq!(back.nodes().get(&a).unwrap().note(), Some("note-a"));
        assert_eq!(back.edges().get(&oid("e:1")).unwrap().connector(), Some("<-->"));
        assert_eq!(back.edges().get(&oid("e:1")).unwrap().label(), Some("go"));
        assert_eq!(back.edges().get(&oid("e:1")).unwrap().style(), Some("dashed"));
    }
}
