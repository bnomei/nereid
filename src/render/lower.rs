// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Kind-specific lowerers: domain AST → graph/track scene models.

use crate::model::class_ast::{ClassAst, ClassRelationKind};
use crate::model::diagram::DiagramAst;
use crate::model::er_ast::{ErAst, ErCardinality, ErStroke};
use crate::model::flow_ast::FlowchartAst;
use crate::model::gantt_ast::GanttAst;
use crate::model::seq_ast::SequenceAst;

use super::scene::{
    CapKind, EdgeStroke, GraphCompartment, GraphEdge, GraphModel, GraphNode, RenderScene,
    TrackModel,
};

/// Lower a flowchart AST into the shared graph scene.
pub fn lower_flowchart(ast: &FlowchartAst) -> GraphModel {
    let mut model = GraphModel::default();

    for (id, node) in ast.nodes() {
        let graph_node = GraphNode::new(node.label())
            .with_mermaid_id(node.mermaid_id().map(str::to_owned))
            .with_shape(node.shape())
            .with_note(node.note().map(str::to_owned))
            .with_symbol(node.symbol().cloned());
        model.nodes_mut().insert(id.clone(), graph_node);
    }

    for (id, edge) in ast.edges() {
        let (start_cap, end_cap) = CapKind::from_flow_connector(edge.connector());
        let stroke = crate::render::scene::EdgeStroke::from_flow_connector_and_style(
            edge.connector(),
            edge.style(),
        );
        let graph_edge = GraphEdge::new(edge.from_node_id().clone(), edge.to_node_id().clone())
            .with_label(edge.label().map(str::to_owned))
            .with_connector(edge.connector().map(str::to_owned))
            .with_style(edge.style().map(str::to_owned))
            .with_caps(start_cap, end_cap)
            .with_stroke(stroke);
        model.edges_mut().insert(id.clone(), graph_edge);
    }

    model
}

/// Lower a sequence AST into the shared track scene.
pub fn lower_sequence(ast: &SequenceAst) -> TrackModel {
    TrackModel::from_sequence(ast.clone())
}

fn class_relation_caps(kind: ClassRelationKind, raw: Option<&str>) -> (CapKind, CapKind) {
    let token = raw.unwrap_or("");
    match kind {
        ClassRelationKind::Inheritance | ClassRelationKind::Realization => {
            if token.starts_with("<|") {
                (CapKind::TriangleHollow, CapKind::None)
            } else {
                (CapKind::None, CapKind::TriangleHollow)
            }
        }
        ClassRelationKind::Composition => {
            if token.starts_with('*') {
                (CapKind::DiamondFilled, CapKind::None)
            } else {
                // `--*` or default: filled diamond at the "to" end.
                (CapKind::None, CapKind::DiamondFilled)
            }
        }
        ClassRelationKind::Aggregation => {
            if token.starts_with('o') {
                (CapKind::DiamondHollow, CapKind::None)
            } else {
                // `--o` or default: hollow diamond at the "to" end.
                (CapKind::None, CapKind::DiamondHollow)
            }
        }
        ClassRelationKind::Association => {
            if token.starts_with('<') && !token.starts_with("<|") {
                (CapKind::Arrow, CapKind::None)
            } else {
                (CapKind::None, CapKind::Arrow)
            }
        }
        ClassRelationKind::Dependency => {
            if token.contains('>') {
                (CapKind::None, CapKind::Arrow)
            } else if token.starts_with('<') {
                (CapKind::Arrow, CapKind::None)
            } else {
                (CapKind::None, CapKind::None)
            }
        }
        ClassRelationKind::Link => {
            if token.contains("<-->") || token == "<-->" {
                (CapKind::Arrow, CapKind::Arrow)
            } else {
                (CapKind::None, CapKind::None)
            }
        }
    }
}

fn class_relation_stroke(kind: ClassRelationKind, raw: Option<&str>) -> EdgeStroke {
    let token = raw.unwrap_or("");
    match kind {
        ClassRelationKind::Dependency | ClassRelationKind::Realization => EdgeStroke::Dashed,
        _ if token.contains('.') => EdgeStroke::Dashed,
        _ => EdgeStroke::Solid,
    }
}

/// Lower a class diagram into the shared graph scene (compartments + relation caps).
pub fn lower_class(ast: &ClassAst) -> GraphModel {
    let mut model = GraphModel::default();

    for (id, class) in ast.classes() {
        let mut compartments = Vec::new();
        // Always emit attribute then method compartments when either has content, matching UML.
        if !class.attributes().is_empty() || !class.methods().is_empty() {
            compartments.push(GraphCompartment::new(class.attributes().iter().cloned()));
            compartments.push(GraphCompartment::new(class.methods().iter().cloned()));
        }
        let node = GraphNode::new(class.name())
            .with_compartments(compartments)
            .with_note(class.note().map(str::to_owned));
        model.nodes_mut().insert(id.clone(), node);
    }

    for (id, rel) in ast.relations() {
        let (start_cap, end_cap) = class_relation_caps(rel.kind(), rel.raw_connector());
        let stroke = class_relation_stroke(rel.kind(), rel.raw_connector());
        // Bridge connector: use a flow-compatible token so existing paint shows arrows when
        // CapKind is not yet fully wired; store semantic caps on the edge for future paint.
        // Prefer raw Mermaid token when present so dashed/no-cap links do not gain false arrows
        // if the model ever falls through to flowchart paint.
        let bridge_connector =
            rel.raw_connector().map(str::to_owned).or_else(|| match (start_cap, end_cap) {
                (CapKind::Arrow, CapKind::Arrow) => Some("<-->".to_owned()),
                (CapKind::Arrow, _) => Some("<--".to_owned()),
                (_, CapKind::Arrow) => Some("-->".to_owned()),
                (CapKind::Circle | CapKind::DiamondHollow | CapKind::ZeroOrOne, _) => {
                    Some("o--".to_owned())
                }
                (_, CapKind::Circle | CapKind::DiamondHollow | CapKind::ZeroOrOne) => {
                    Some("--o".to_owned())
                }
                (CapKind::None, CapKind::None) => Some("---".to_owned()),
                _ => Some("---".to_owned()),
            });
        let edge = GraphEdge::new(rel.from_class_id().clone(), rel.to_class_id().clone())
            .with_label(rel.label().map(str::to_owned))
            .with_connector(bridge_connector)
            .with_caps(start_cap, end_cap)
            .with_stroke(stroke);
        model.edges_mut().insert(id.clone(), edge);
    }

    model
}

fn er_card_to_cap(card: ErCardinality) -> CapKind {
    match card {
        ErCardinality::ExactlyOne => CapKind::ExactlyOne,
        ErCardinality::ZeroOrOne => CapKind::ZeroOrOne,
        ErCardinality::OneOrMore => CapKind::CrowFoot,
        ErCardinality::ZeroOrMore => CapKind::ZeroOrMore,
    }
}

/// Lower ER diagram into graph scene (plain entity boxes + folded 1-cell cardinality caps).
pub fn lower_er(ast: &ErAst) -> GraphModel {
    let mut model = GraphModel::default();
    for (id, entity) in ast.entities() {
        let node = GraphNode::new(entity.name()).with_note(entity.note().map(str::to_owned));
        model.nodes_mut().insert(id.clone(), node);
    }
    for (id, rel) in ast.relationships() {
        let start_cap = er_card_to_cap(rel.from_card());
        let end_cap = er_card_to_cap(rel.to_card());
        let stroke = match rel.stroke() {
            ErStroke::Identifying => EdgeStroke::Solid,
            ErStroke::NonIdentifying => EdgeStroke::Dashed,
        };
        // Prefer raw Mermaid token; never invent arrow heads for pure cardinality ends.
        let bridge =
            rel.raw_connector().map(str::to_owned).or_else(|| match (start_cap, end_cap) {
                (CapKind::ZeroOrOne | CapKind::ZeroOrMore, _) => Some("o--".to_owned()),
                (_, CapKind::ZeroOrOne | CapKind::ZeroOrMore) => Some("--o".to_owned()),
                _ => Some("---".to_owned()),
            });
        let edge = GraphEdge::new(rel.from_entity_id().clone(), rel.to_entity_id().clone())
            .with_label(rel.label().map(str::to_owned))
            .with_connector(bridge)
            .with_caps(start_cap, end_cap)
            .with_stroke(stroke);
        model.edges_mut().insert(id.clone(), edge);
    }
    model
}

/// Lower gantt into track scene (domain AST retained for multi-col bar paint).
pub fn lower_gantt(ast: &GanttAst) -> TrackModel {
    TrackModel::from_gantt(ast.clone())
}

/// Lower any supported diagram AST into a render scene.
pub fn lower_diagram_ast(ast: &DiagramAst) -> RenderScene {
    match ast {
        DiagramAst::Flowchart(flow) => RenderScene::Graph(lower_flowchart(flow)),
        DiagramAst::Sequence(seq) => RenderScene::Track(lower_sequence(seq)),
        DiagramAst::Class(class) => RenderScene::Graph(lower_class(class)),
        DiagramAst::Er(er) => RenderScene::Graph(lower_er(er)),
        DiagramAst::Gantt(gantt) => RenderScene::Track(lower_gantt(gantt)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::flow_ast::{FlowEdge, FlowNode};
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{SequenceAst, SequenceParticipant};
    use crate::render::scene::CapKind;

    fn oid(s: &str) -> ObjectId {
        ObjectId::new(s).expect("object id")
    }

    #[test]
    fn lower_flowchart_resolves_caps() {
        let mut ast = FlowchartAst::default();
        let a = oid("n:a");
        let b = oid("n:b");
        ast.nodes_mut().insert(a.clone(), FlowNode::new("A"));
        ast.nodes_mut().insert(b.clone(), FlowNode::new("B"));
        let mut edge = FlowEdge::new(a, b);
        edge.set_connector(Some("o-->".to_owned()));
        ast.edges_mut().insert(oid("e:1"), edge);

        let model = lower_flowchart(&ast);
        let e = model.edges().get(&oid("e:1")).expect("edge");
        assert_eq!(e.start_cap(), CapKind::Circle);
        assert_eq!(e.end_cap(), CapKind::Arrow);
    }

    #[test]
    fn lower_sequence_preserves_participants() {
        let mut ast = SequenceAst::default();
        ast.participants_mut().insert(oid("p:a"), SequenceParticipant::new("Alice"));
        let track = lower_sequence(&ast);
        assert_eq!(track.as_sequence_ast().participants().len(), 1);
        assert_eq!(track.default_span_style(), crate::render::scene::TrackSpanStyle::Arrow);
    }

    #[test]
    fn lower_diagram_ast_picks_family() {
        let scene = lower_diagram_ast(&DiagramAst::Flowchart(FlowchartAst::default()));
        assert_eq!(scene.family_name(), "graph");
        let scene = lower_diagram_ast(&DiagramAst::Sequence(SequenceAst::default()));
        assert_eq!(scene.family_name(), "track");
    }

    #[test]
    fn default_class_relation_caps_survive_export_roundtrip() {
        let mut ast = ClassAst::default();
        ast.classes_mut().insert(oid("c:a"), crate::model::ClassNode::new("A"));
        ast.classes_mut().insert(oid("c:b"), crate::model::ClassNode::new("B"));
        ast.relations_mut().insert(
            oid("r:1"),
            crate::model::ClassRelation::new(
                oid("c:a"),
                oid("c:b"),
                ClassRelationKind::Composition,
            ),
        );

        let before = lower_class(&ast);
        let exported = crate::format::mermaid::export_class_diagram(&ast).expect("export");
        let reparsed = crate::format::mermaid::parse_class_diagram(&exported).expect("parse");
        let after = lower_class(&reparsed);
        let before_edge = before.edges().values().next().unwrap();
        let after_edge = after.edges().values().next().unwrap();
        assert_eq!(before_edge.start_cap(), after_edge.start_cap());
        assert_eq!(before_edge.end_cap(), after_edge.end_cap());
    }

    #[test]
    fn er_one_or_more_and_zero_or_more_lower_to_distinct_caps() {
        assert_eq!(er_card_to_cap(ErCardinality::OneOrMore), CapKind::CrowFoot);
        assert_eq!(er_card_to_cap(ErCardinality::ZeroOrMore), CapKind::ZeroOrMore);
    }
}
