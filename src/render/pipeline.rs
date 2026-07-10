// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Coherent render pipeline: scene → layout family → paint.
//!
//! Kind-specific code stops at lowering ([`super::lower`]). This module owns family dispatch so
//! new diagram kinds do not grow parallel layout/render stacks.

use std::fmt;

use crate::layout::{layout_graph, layout_track, FlowchartLayoutError, SequenceLayoutError};
use crate::model::diagram::DiagramAst;
use crate::model::ids::DiagramId;

use super::flowchart::{
    render_flowchart_unicode_annotated_with_options, render_flowchart_unicode_with_options,
    FlowchartRenderError,
};
use super::lower::lower_diagram_ast;
use super::scene::{GraphModel, RenderScene, TrackModel};
use super::sequence::{
    render_sequence_unicode_annotated_with_options, render_sequence_unicode_with_options,
    SequenceRenderError,
};
use super::{AnnotatedRender, CanvasError, RenderOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineRenderError {
    SequenceLayout(SequenceLayoutError),
    FlowchartLayout(FlowchartLayoutError),
    SequenceRender(SequenceRenderError),
    FlowchartRender(FlowchartRenderError),
    Canvas(CanvasError),
}

impl fmt::Display for PipelineRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SequenceLayout(err) => write!(f, "sequence layout error: {err}"),
            Self::FlowchartLayout(err) => write!(f, "flowchart layout error: {err}"),
            Self::SequenceRender(err) => write!(f, "sequence render error: {err}"),
            Self::FlowchartRender(err) => write!(f, "flowchart render error: {err}"),
            Self::Canvas(err) => write!(f, "canvas error: {err}"),
        }
    }
}

impl std::error::Error for PipelineRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SequenceLayout(err) => Some(err),
            Self::FlowchartLayout(err) => Some(err),
            Self::SequenceRender(err) => Some(err),
            Self::FlowchartRender(err) => Some(err),
            Self::Canvas(err) => Some(err),
        }
    }
}

impl From<CanvasError> for PipelineRenderError {
    fn from(value: CanvasError) -> Self {
        Self::Canvas(value)
    }
}

impl From<SequenceLayoutError> for PipelineRenderError {
    fn from(value: SequenceLayoutError) -> Self {
        Self::SequenceLayout(value)
    }
}

impl From<FlowchartLayoutError> for PipelineRenderError {
    fn from(value: FlowchartLayoutError) -> Self {
        Self::FlowchartLayout(value)
    }
}

impl From<SequenceRenderError> for PipelineRenderError {
    fn from(value: SequenceRenderError) -> Self {
        Self::SequenceRender(value)
    }
}

impl From<FlowchartRenderError> for PipelineRenderError {
    fn from(value: FlowchartRenderError) -> Self {
        Self::FlowchartRender(value)
    }
}

/// Layout + paint a graph-family scene (flow / class / er).
///
/// Uses [`layout_graph`] then bridges to the existing flowchart painter via
/// [`GraphModel::to_flowchart_ast`] (temporary until graph paint is scene-native).
///
/// Cap note: flowchart helpers still parse connector strings for glyphs; `GraphEdge` caps are
/// resolved at lower time for upcoming scene-native paint and must stay in sync with connectors.
pub fn render_graph_unicode_with_options(
    model: &GraphModel,
    options: RenderOptions,
) -> Result<String, PipelineRenderError> {
    let layout = layout_graph(model)?;
    if crate::render::graph_paint::graph_model_needs_scene_paint(model) {
        return Ok(crate::render::graph_paint::render_graph_model_with_compartments(
            model, &layout, options,
        )?);
    }
    let ast = model.to_flowchart_ast();
    Ok(render_flowchart_unicode_with_options(&ast, &layout, options)?)
}

/// Annotated graph-family render.
///
/// When `highlight_categories` is set (class/er scene paint), builds a real `HighlightIndex` for
/// TUI hints. Flow-bridge path ignores it and uses flowchart categories.
pub fn render_graph_unicode_annotated_with_options(
    diagram_id: &DiagramId,
    model: &GraphModel,
    options: RenderOptions,
) -> Result<AnnotatedRender, PipelineRenderError> {
    render_graph_unicode_annotated_with_categories(diagram_id, model, options, None)
}

/// Annotated graph paint with explicit node/edge category segments for scene-native diagrams.
///
/// When `highlight_categories` is `Some` (class/ER entry points), always uses scene-native paint
/// so dashed strokes and `class/*`/`er/*` refs are never lost to the flowchart bridge.
pub fn render_graph_unicode_annotated_with_categories(
    diagram_id: &DiagramId,
    model: &GraphModel,
    options: RenderOptions,
    highlight_categories: Option<crate::render::graph_paint::GraphHighlightCategories<'_>>,
) -> Result<AnnotatedRender, PipelineRenderError> {
    let layout = layout_graph(model)?;
    if let Some(categories) = highlight_categories {
        return Ok(crate::render::graph_paint::render_graph_model_with_compartments_annotated(
            diagram_id, model, &layout, options, categories,
        )?);
    }
    if crate::render::graph_paint::graph_model_needs_scene_paint(model) {
        return Ok(crate::render::graph_paint::render_graph_model_with_compartments_annotated(
            diagram_id,
            model,
            &layout,
            options,
            crate::render::graph_paint::GraphHighlightCategories::FLOW,
        )?);
    }
    let ast = model.to_flowchart_ast();
    Ok(render_flowchart_unicode_annotated_with_options(diagram_id, &ast, &layout, options)?)
}

/// Class diagrams always use scene-native paint (dashed links, compartments, class/* hints).
pub fn render_class_unicode_with_options(
    model: &GraphModel,
    options: RenderOptions,
) -> Result<String, PipelineRenderError> {
    let layout = layout_graph(model)?;
    Ok(crate::render::graph_paint::render_graph_model_with_compartments(model, &layout, options)?)
}

/// ER diagrams always use scene-native paint (cardinality caps, er/* hints).
pub fn render_er_unicode_with_options(
    model: &GraphModel,
    options: RenderOptions,
) -> Result<String, PipelineRenderError> {
    let layout = layout_graph(model)?;
    Ok(crate::render::graph_paint::render_graph_model_with_compartments(model, &layout, options)?)
}

/// Layout + paint a track-family scene (sequence / gantt).
pub fn render_track_unicode_with_options(
    model: &TrackModel,
    options: RenderOptions,
) -> Result<String, PipelineRenderError> {
    if let Some(gantt) = model.as_gantt_ast() {
        return Ok(crate::render::track_paint::render_gantt_unicode(gantt, options)?);
    }
    let layout = layout_track(model)?;
    let ast = model.as_sequence_ast();
    Ok(render_sequence_unicode_with_options(ast, &layout, options)?)
}

/// Annotated track-family render.
pub fn render_track_unicode_annotated_with_options(
    diagram_id: &DiagramId,
    model: &TrackModel,
    options: RenderOptions,
) -> Result<AnnotatedRender, PipelineRenderError> {
    if let Some(gantt) = model.as_gantt_ast() {
        return Ok(crate::render::track_paint::render_gantt_unicode_annotated(
            diagram_id, gantt, options,
        )?);
    }
    let layout = layout_track(model)?;
    let ast = model.as_sequence_ast();
    Ok(render_sequence_unicode_annotated_with_options(diagram_id, ast, &layout, options)?)
}

/// Family-dispatched paint for a lowered scene.
pub fn render_scene_unicode_with_options(
    scene: &RenderScene,
    options: RenderOptions,
) -> Result<String, PipelineRenderError> {
    match scene {
        RenderScene::Graph(model) => render_graph_unicode_with_options(model, options),
        RenderScene::Track(model) => render_track_unicode_with_options(model, options),
    }
}

/// Family-dispatched annotated paint for a lowered scene.
pub fn render_scene_unicode_annotated_with_options(
    diagram_id: &DiagramId,
    scene: &RenderScene,
    options: RenderOptions,
) -> Result<AnnotatedRender, PipelineRenderError> {
    match scene {
        RenderScene::Graph(model) => {
            render_graph_unicode_annotated_with_options(diagram_id, model, options)
        }
        RenderScene::Track(model) => {
            render_track_unicode_annotated_with_options(diagram_id, model, options)
        }
    }
}

/// Lower a domain AST then render through the coherent pipeline.
pub fn render_ast_unicode_with_options(
    ast: &DiagramAst,
    options: RenderOptions,
) -> Result<String, PipelineRenderError> {
    match ast {
        // Always scene-native so dashed links and non-flow caps cannot fall through to flowchart.
        DiagramAst::Class(class) => {
            let model = crate::render::lower::lower_class(class);
            render_class_unicode_with_options(&model, options)
        }
        DiagramAst::Er(er) => {
            let model = crate::render::lower::lower_er(er);
            render_er_unicode_with_options(&model, options)
        }
        other => {
            let scene = lower_diagram_ast(other);
            render_scene_unicode_with_options(&scene, options)
        }
    }
}

/// Lower a domain AST then annotated-render through the coherent pipeline.
pub fn render_ast_unicode_annotated_with_options(
    diagram_id: &DiagramId,
    ast: &DiagramAst,
    options: RenderOptions,
) -> Result<AnnotatedRender, PipelineRenderError> {
    use crate::render::graph_paint::GraphHighlightCategories;

    match ast {
        DiagramAst::Class(class) => {
            let model = crate::render::lower::lower_class(class);
            render_graph_unicode_annotated_with_categories(
                diagram_id,
                &model,
                options,
                Some(GraphHighlightCategories::CLASS),
            )
        }
        DiagramAst::Er(er) => {
            let model = crate::render::lower::lower_er(er);
            render_graph_unicode_annotated_with_categories(
                diagram_id,
                &model,
                options,
                Some(GraphHighlightCategories::ER),
            )
        }
        other => {
            let scene = lower_diagram_ast(other);
            render_scene_unicode_annotated_with_options(diagram_id, &scene, options)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{layout_flowchart, layout_sequence};
    use crate::model::fixtures::flowchart_small_dag;
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    };
    use crate::model::{Diagram, DiagramAst, DiagramId};
    use crate::render::flowchart::render_flowchart_unicode;
    use crate::render::sequence::render_sequence_unicode;

    fn oid(value: &str) -> ObjectId {
        ObjectId::new(value).expect("object id")
    }

    #[test]
    fn pipeline_flowchart_matches_direct_render() {
        let ast = flowchart_small_dag();
        let layout = layout_flowchart(&ast).expect("layout");
        let direct = render_flowchart_unicode(&ast, &layout).expect("direct");
        let via =
            render_ast_unicode_with_options(&DiagramAst::Flowchart(ast), RenderOptions::default())
                .expect("pipeline");
        assert_eq!(via, direct);
    }

    #[test]
    fn pipeline_sequence_matches_direct_render() {
        let mut ast = SequenceAst::default();
        let p_alice = oid("p:alice");
        let p_bob = oid("p:bob");
        ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
        ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
        ast.messages_mut().push(SequenceMessage::new(
            oid("m:0001"),
            p_alice,
            p_bob,
            SequenceMessageKind::Sync,
            "Hello",
            1000,
        ));

        let layout = layout_sequence(&ast).expect("layout");
        let direct = render_sequence_unicode(&ast, &layout).expect("direct");
        let via =
            render_ast_unicode_with_options(&DiagramAst::Sequence(ast), RenderOptions::default())
                .expect("pipeline");
        assert_eq!(via, direct);
    }

    #[test]
    fn both_families_enter_shared_pipeline_entry() {
        let flow = DiagramAst::Flowchart(flowchart_small_dag());
        let mut seq = SequenceAst::default();
        seq.participants_mut().insert(oid("p:a"), SequenceParticipant::new("A"));
        let seq = DiagramAst::Sequence(seq);

        let flow_scene = lower_diagram_ast(&flow);
        let seq_scene = lower_diagram_ast(&seq);
        assert_eq!(flow_scene.family_name(), "graph");
        assert_eq!(seq_scene.family_name(), "track");

        let _ = render_scene_unicode_with_options(&flow_scene, RenderOptions::default())
            .expect("graph paint");
        let _ = render_scene_unicode_with_options(&seq_scene, RenderOptions::default())
            .expect("track paint");
    }

    #[test]
    fn diagram_dispatch_uses_pipeline_parity() {
        let flow_ast = flowchart_small_dag();
        let diagram_id = DiagramId::new("d-flow").expect("id");
        let diagram = Diagram::new(diagram_id, "Example", DiagramAst::Flowchart(flow_ast.clone()));

        let via_diagram = crate::render::render_diagram_unicode(&diagram).expect("diagram");
        let via_pipeline = render_ast_unicode_with_options(
            &DiagramAst::Flowchart(flow_ast),
            RenderOptions::default(),
        )
        .expect("pipeline");
        assert_eq!(via_diagram, via_pipeline);
    }

    #[test]
    fn layout_graph_matches_layout_flowchart() {
        let ast = flowchart_small_dag();
        let direct = layout_flowchart(&ast).expect("direct layout");
        let via = layout_graph(&crate::render::lower_flowchart(&ast)).expect("graph layout");
        assert_eq!(via, direct);
    }

    #[test]
    fn pipeline_annotated_flowchart_matches_direct() {
        use crate::render::flowchart::render_flowchart_unicode_annotated;
        let ast = flowchart_small_dag();
        let diagram_id = DiagramId::new("d-ann").expect("id");
        let layout = layout_flowchart(&ast).expect("layout");
        let direct =
            render_flowchart_unicode_annotated(&diagram_id, &ast, &layout).expect("direct");
        let via = render_ast_unicode_annotated_with_options(
            &diagram_id,
            &DiagramAst::Flowchart(ast),
            RenderOptions::default(),
        )
        .expect("pipeline");
        assert_eq!(via.text, direct.text);
        assert_eq!(via.highlight_index, direct.highlight_index);
    }

    #[test]
    fn pipeline_dual_cap_connector_matches_direct() {
        use crate::model::flow_ast::{FlowEdge, FlowNode};
        let mut ast = crate::model::flow_ast::FlowchartAst::default();
        let a = oid("n:a");
        let b = oid("n:b");
        ast.nodes_mut().insert(a.clone(), FlowNode::new("A"));
        ast.nodes_mut().insert(b.clone(), FlowNode::new("B"));
        let mut edge = FlowEdge::new(a, b);
        edge.set_connector(Some("<-->".to_owned()));
        ast.edges_mut().insert(oid("e:1"), edge);

        let layout = layout_flowchart(&ast).expect("layout");
        let direct = render_flowchart_unicode(&ast, &layout).expect("direct");
        let via =
            render_ast_unicode_with_options(&DiagramAst::Flowchart(ast), RenderOptions::default())
                .expect("pipeline");
        assert_eq!(via, direct);
        assert!(via.contains('◀') || via.contains('▶'), "expected dual caps:\n{via}");
    }

    #[test]
    fn class_diagram_renders_compartments_via_pipeline() {
        let input = r#"
classDiagram
Class01 <|-- AveryLongClass : Cool
Class01 : size()
Class01 : int chimp
Class03 *-- Class04
"#;
        let ast = crate::format::mermaid::parse_class_diagram(input).expect("parse");
        let text =
            render_ast_unicode_with_options(&DiagramAst::Class(ast), RenderOptions::default())
                .expect("render");
        assert!(text.contains("Class01"), "{text}");
        assert!(text.contains("int chimp") || text.contains("size()"), "{text}");
        assert!(text.contains('├') || text.contains('─'), "expected class box:\n{text}");
    }

    #[test]
    fn pipeline_notes_on_matches_direct() {
        use crate::model::flow_ast::FlowNode;
        let mut ast = crate::model::flow_ast::FlowchartAst::default();
        let a = oid("n:a");
        let mut node = FlowNode::new("A");
        node.set_note(Some("note".to_owned()));
        ast.nodes_mut().insert(a, node);
        let options = RenderOptions { show_notes: true, ..RenderOptions::default() };
        let layout = layout_flowchart(&ast).expect("layout");
        let direct =
            crate::render::flowchart::render_flowchart_unicode_with_options(&ast, &layout, options)
                .expect("direct");
        let via = render_ast_unicode_with_options(&DiagramAst::Flowchart(ast), options)
            .expect("pipeline");
        assert_eq!(via, direct);
        // Note may be ellipsis-truncated inside a narrow box ("no…").
        assert!(
            via.lines().any(|line| line.contains('n') && line.contains('│')),
            "expected note row inside box:\n{via}"
        );
    }
}
