// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::flow_ast::FlowchartAst;
use super::ids::DiagramId;
use super::seq_ast::SequenceAst;
use std::fmt;

/// The type of diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramKind {
    Sequence,
    Flowchart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramAst {
    Sequence(SequenceAst),
    Flowchart(FlowchartAst),
}

impl DiagramAst {
    pub fn kind(&self) -> DiagramKind {
        match self {
            Self::Sequence(_) => DiagramKind::Sequence,
            Self::Flowchart(_) => DiagramKind::Flowchart,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagramAstKindMismatch {
    expected: DiagramKind,
    found: DiagramKind,
}

impl DiagramAstKindMismatch {
    pub fn expected(&self) -> DiagramKind {
        self.expected
    }

    pub fn found(&self) -> DiagramKind {
        self.found
    }
}

impl fmt::Display for DiagramAstKindMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "diagram ast kind mismatch (expected {:?}, found {:?})",
            self.expected, self.found
        )
    }
}

impl std::error::Error for DiagramAstKindMismatch {}

/// A single, typed diagram artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagram {
    diagram_id: DiagramId,
    name: String,
    kind: DiagramKind,
    ast: DiagramAst,
    rev: u64,
}

impl Diagram {
    pub fn new(diagram_id: DiagramId, name: impl Into<String>, ast: DiagramAst) -> Self {
        let kind = ast.kind();
        Self {
            diagram_id,
            name: name.into(),
            kind,
            ast,
            rev: 0,
        }
    }

    pub fn diagram_id(&self) -> &DiagramId {
        &self.diagram_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> DiagramKind {
        self.kind
    }

    pub fn ast(&self) -> &DiagramAst {
        &self.ast
    }

    pub fn replace_ast(&mut self, ast: DiagramAst) -> Result<DiagramAst, DiagramAstKindMismatch> {
        let found = ast.kind();
        if found != self.kind {
            return Err(DiagramAstKindMismatch {
                expected: self.kind,
                found,
            });
        }

        Ok(std::mem::replace(&mut self.ast, ast))
    }

    pub fn set_ast(&mut self, ast: DiagramAst) -> Result<(), DiagramAstKindMismatch> {
        self.replace_ast(ast).map(|_| ())
    }

    pub fn rev(&self) -> u64 {
        self.rev
    }

    pub fn set_rev(&mut self, rev: u64) {
        self.rev = rev;
    }

    pub fn bump_rev(&mut self) {
        self.rev = self.rev.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagram, DiagramAst, DiagramAstKindMismatch, DiagramKind};
    use crate::model::{DiagramId, FlowchartAst, SequenceAst};

    #[test]
    fn diagram_can_replace_ast_without_resetting_rev() {
        let diagram_id = DiagramId::new("d1").expect("diagram id");
        let mut diagram = Diagram::new(
            diagram_id.clone(),
            "Example",
            DiagramAst::Sequence(SequenceAst::default()),
        );

        diagram.bump_rev();
        diagram.bump_rev();

        diagram
            .set_ast(DiagramAst::Sequence(SequenceAst::default()))
            .expect("set_ast");

        assert_eq!(diagram.diagram_id(), &diagram_id);
        assert_eq!(diagram.name(), "Example");
        assert_eq!(diagram.kind(), DiagramKind::Sequence);
        assert_eq!(diagram.rev(), 2);

        diagram.bump_rev();
        assert_eq!(diagram.rev(), 3);
    }

    #[test]
    fn diagram_rejects_replacing_ast_with_different_kind() {
        let diagram_id = DiagramId::new("d1").expect("diagram id");
        let mut diagram = Diagram::new(
            diagram_id,
            "Example",
            DiagramAst::Sequence(SequenceAst::default()),
        );

        let result = diagram.replace_ast(DiagramAst::Flowchart(FlowchartAst::default()));

        assert_eq!(
            result,
            Err(DiagramAstKindMismatch {
                expected: DiagramKind::Sequence,
                found: DiagramKind::Flowchart,
            })
        );

        assert_eq!(diagram.kind(), DiagramKind::Sequence);
        assert_eq!(diagram.ast().kind(), DiagramKind::Sequence);
    }
}
