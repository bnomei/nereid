// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Kind-specific lowerers: domain AST → graph/track scene models.

use crate::model::diagram::DiagramAst;
use crate::model::flow_ast::FlowchartAst;
use crate::model::seq_ast::SequenceAst;

use super::scene::{CapKind, GraphEdge, GraphModel, GraphNode, RenderScene, TrackModel};

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

/// Lower any supported diagram AST into a render scene.
pub fn lower_diagram_ast(ast: &DiagramAst) -> RenderScene {
    match ast {
        DiagramAst::Flowchart(flow) => RenderScene::Graph(lower_flowchart(flow)),
        DiagramAst::Sequence(seq) => RenderScene::Track(lower_sequence(seq)),
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
}
