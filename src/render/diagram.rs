// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::fmt;

use crate::layout::{layout_flowchart, layout_sequence, FlowchartLayoutError, SequenceLayoutError};
use crate::model::diagram::{Diagram, DiagramAst};

use super::flowchart::{
    render_flowchart_unicode_annotated_with_options, render_flowchart_unicode_with_options,
    FlowchartRenderError,
};
use super::sequence::{
    render_sequence_unicode_annotated_with_options, render_sequence_unicode_with_options,
    SequenceRenderError,
};
use super::{AnnotatedRender, RenderOptions};

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

pub fn render_diagram_unicode(diagram: &Diagram) -> Result<String, DiagramRenderError> {
    render_diagram_unicode_with_options(diagram, RenderOptions::default())
}

pub fn render_diagram_unicode_with_options(
    diagram: &Diagram,
    options: RenderOptions,
) -> Result<String, DiagramRenderError> {
    match diagram.ast() {
        DiagramAst::Sequence(ast) => {
            let layout = layout_sequence(ast)?;
            Ok(render_sequence_unicode_with_options(ast, &layout, options)?)
        }
        DiagramAst::Flowchart(ast) => {
            let layout = layout_flowchart(ast)?;
            Ok(render_flowchart_unicode_with_options(ast, &layout, options)?)
        }
    }
}

pub fn render_diagram_unicode_annotated(
    diagram: &Diagram,
) -> Result<AnnotatedRender, DiagramRenderError> {
    render_diagram_unicode_annotated_with_options(diagram, RenderOptions::default())
}

pub fn render_diagram_unicode_annotated_with_options(
    diagram: &Diagram,
    options: RenderOptions,
) -> Result<AnnotatedRender, DiagramRenderError> {
    match diagram.ast() {
        DiagramAst::Sequence(ast) => {
            let layout = layout_sequence(ast)?;
            Ok(render_sequence_unicode_annotated_with_options(
                diagram.diagram_id(),
                ast,
                &layout,
                options,
            )?)
        }
        DiagramAst::Flowchart(ast) => {
            let layout = layout_flowchart(ast)?;
            Ok(render_flowchart_unicode_annotated_with_options(
                diagram.diagram_id(),
                ast,
                &layout,
                options,
            )?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render_diagram_unicode;
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    };
    use crate::model::{Diagram, DiagramAst, DiagramId};

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
    }

    #[test]
    fn snapshot_flowchart_via_render_diagram_unicode() {
        let ast = crate::model::fixtures::flowchart_small_dag();

        let diagram_id = DiagramId::new("d-flow").expect("diagram id");
        let diagram = Diagram::new(diagram_id, "Example", DiagramAst::Flowchart(ast));
        let rendered = render_diagram_unicode(&diagram).expect("render");

        assert_eq!(
            rendered,
            "┌───┐   ┌───┐   ┌───┐\n│ A ├──┬┤ B ├──┬┤ D │\n└───┘  │└───┘  │└───┘\n       │       │\n       │       │\n       │┌───┐  │\n       └┤ C ├──┘\n        └───┘"
        );
    }
}
