// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Kind-dispatching diagram renderer via the coherent scene pipeline.
//!
//! Domain ASTs lower to graph/track scenes; family layout + paint live in [`super::pipeline`].

use std::fmt;

use crate::layout::{FlowchartLayoutError, SequenceLayoutError};
use crate::model::diagram::Diagram;

use super::flowchart::FlowchartRenderError;
use super::pipeline::{
    render_ast_unicode_annotated_with_options, render_ast_unicode_with_options, PipelineRenderError,
};
use super::sequence::SequenceRenderError;
use super::{AnnotatedRender, RenderOptions};

/// Errors from layout or paint when rendering a full [`Diagram`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramRenderError {
    SequenceLayout(SequenceLayoutError),
    FlowchartLayout(FlowchartLayoutError),
    SequenceRender(SequenceRenderError),
    FlowchartRender(FlowchartRenderError),
}

impl fmt::Display for DiagramRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SequenceLayout(err) => write!(f, "sequence layout error: {err}"),
            Self::FlowchartLayout(err) => write!(f, "flowchart layout error: {err}"),
            Self::SequenceRender(err) => write!(f, "sequence render error: {err}"),
            Self::FlowchartRender(err) => write!(f, "flowchart render error: {err}"),
        }
    }
}

impl std::error::Error for DiagramRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SequenceLayout(err) => Some(err),
            Self::FlowchartLayout(err) => Some(err),
            Self::SequenceRender(err) => Some(err),
            Self::FlowchartRender(err) => Some(err),
        }
    }
}

impl From<SequenceLayoutError> for DiagramRenderError {
    fn from(value: SequenceLayoutError) -> Self {
        Self::SequenceLayout(value)
    }
}

impl From<FlowchartLayoutError> for DiagramRenderError {
    fn from(value: FlowchartLayoutError) -> Self {
        Self::FlowchartLayout(value)
    }
}

impl From<SequenceRenderError> for DiagramRenderError {
    fn from(value: SequenceRenderError) -> Self {
        Self::SequenceRender(value)
    }
}

impl From<FlowchartRenderError> for DiagramRenderError {
    fn from(value: FlowchartRenderError) -> Self {
        Self::FlowchartRender(value)
    }
}

impl From<PipelineRenderError> for DiagramRenderError {
    fn from(value: PipelineRenderError) -> Self {
        match value {
            PipelineRenderError::SequenceLayout(err) => Self::SequenceLayout(err),
            PipelineRenderError::FlowchartLayout(err) => Self::FlowchartLayout(err),
            PipelineRenderError::SequenceRender(err) => Self::SequenceRender(err),
            PipelineRenderError::FlowchartRender(err) => Self::FlowchartRender(err),
            PipelineRenderError::Canvas(err) => {
                Self::FlowchartRender(crate::render::flowchart::FlowchartRenderError::Canvas(err))
            }
        }
    }
}

/// Primary Unicode text view of a diagram (TUI canvas, MCP `diagram_render_text`).
pub fn render_diagram_unicode(diagram: &Diagram) -> Result<String, DiagramRenderError> {
    render_diagram_unicode_with_options(diagram, RenderOptions::default())
}

/// Like [`render_diagram_unicode`] with notes / labels / flowchart gap toggles.
pub fn render_diagram_unicode_with_options(
    diagram: &Diagram,
    options: RenderOptions,
) -> Result<String, DiagramRenderError> {
    Ok(render_ast_unicode_with_options(diagram.ast(), options)?)
}

/// Unicode render plus per-object cell spans for selection and agent highlight.
pub fn render_diagram_unicode_annotated(
    diagram: &Diagram,
) -> Result<AnnotatedRender, DiagramRenderError> {
    render_diagram_unicode_annotated_with_options(diagram, RenderOptions::default())
}

/// Annotated diagram render with explicit [`RenderOptions`].
pub fn render_diagram_unicode_annotated_with_options(
    diagram: &Diagram,
    options: RenderOptions,
) -> Result<AnnotatedRender, DiagramRenderError> {
    Ok(render_ast_unicode_annotated_with_options(diagram.diagram_id(), diagram.ast(), options)?)
}

#[cfg(test)]
mod tests {
    use super::render_diagram_unicode;
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    };
    use crate::model::{Diagram, DiagramAst, DiagramId};
    use crate::render::lower::lower_diagram_ast;

    fn oid(value: &str) -> ObjectId {
        ObjectId::new(value).expect("object id")
    }

    #[test]
    fn snapshot_sequence_via_render_diagram_unicode() {
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

        let diagram_id = DiagramId::new("d-seq").expect("diagram id");
        let diagram = Diagram::new(diagram_id, "Example", DiagramAst::Sequence(ast));
        let rendered = render_diagram_unicode(&diagram).expect("render");

        assert_eq!(
            rendered,
            " ┌───────┐        ┌─────┐\n │ Alice │        │ Bob │\n └───────┘        └─────┘\n     │               │\n     │               │\n     ├────Hello─────▶│\n     │               │"
        );
        assert_eq!(lower_diagram_ast(diagram.ast()).family_name(), "track");
    }

    #[test]
    fn snapshot_flowchart_via_render_diagram_unicode() {
        let ast = crate::model::fixtures::flowchart_small_dag();

        let diagram_id = DiagramId::new("d-flow").expect("diagram id");
        let diagram = Diagram::new(diagram_id, "Example", DiagramAst::Flowchart(ast));
        let rendered = render_diagram_unicode(&diagram).expect("render");

        assert_eq!(
            rendered,
            "┌───┐   ┌───┐   ┌───┐\n│ A ├──▶│ B ├──▶│ D │\n└───┘  │└───┘  │└───┘\n       │       │\n       │       │\n       │┌───┐  │\n       └┤ C ├──┘\n        └───┘"
        );
        assert_eq!(lower_diagram_ast(diagram.ast()).family_name(), "graph");
    }
}
