// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Diagram envelope: kind, revision, default symbol repository, and typed AST.
//!
//! [`Diagram::rev`] is the optimistic-concurrency (OCC) token: MCP/TUI mutations supply
//! `base_rev` and callers bump on successful write. Kind is fixed at construction; AST replace
//! rejects cross-kind swaps.

use super::class_ast::ClassAst;
use super::er_ast::ErAst;
use super::flow_ast::FlowchartAst;
use super::gantt_ast::GanttAst;
use super::ids::DiagramId;
use super::seq_ast::SequenceAst;
use std::fmt;

/// Supported diagram kinds stored in a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramKind {
    Sequence,
    Flowchart,
    Class,
    Er,
    Gantt,
}

/// Kind-tagged diagram AST payload held by [`Diagram`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramAst {
    Sequence(SequenceAst),
    Flowchart(FlowchartAst),
    Class(ClassAst),
    Er(ErAst),
    Gantt(GanttAst),
}

impl DiagramAst {
    /// Discriminant matching the enclosed AST variant.
    pub fn kind(&self) -> DiagramKind {
        match self {
            Self::Sequence(_) => DiagramKind::Sequence,
            Self::Flowchart(_) => DiagramKind::Flowchart,
            Self::Class(_) => DiagramKind::Class,
            Self::Er(_) => DiagramKind::Er,
            Self::Gantt(_) => DiagramKind::Gantt,
        }
    }
}

/// AST replace attempted with a different [`DiagramKind`] than the diagram was created with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagramAstKindMismatch {
    expected: DiagramKind,
    found: DiagramKind,
}

impl DiagramAstKindMismatch {
    /// Kind fixed on the diagram envelope.
    pub fn expected(&self) -> DiagramKind {
        self.expected
    }

    /// Kind of the AST that was rejected.
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

/// Session-owned diagram artifact: identity, display name, kind-locked AST, and OCC rev.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagram {
    diagram_id: DiagramId,
    name: String,
    kind: DiagramKind,
    ast: DiagramAst,
    rev: u64,
    default_symbol_repository_id: Option<String>,
}

impl Diagram {
    /// Create with `rev = 0`; kind is taken from `ast` and never changes afterward.
    pub fn new(diagram_id: DiagramId, name: impl Into<String>, ast: DiagramAst) -> Self {
        let kind = ast.kind();
        Self {
            diagram_id,
            name: name.into(),
            kind,
            ast,
            rev: 0,
            default_symbol_repository_id: None,
        }
    }

    /// Stable diagram id within the session.
    pub fn diagram_id(&self) -> &DiagramId {
        &self.diagram_id
    }

    /// Human-facing diagram title (not necessarily equal to id).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Kind fixed at construction; drives AST replace checks and tool dispatch.
    pub fn kind(&self) -> DiagramKind {
        self.kind
    }

    /// Borrow the typed AST payload.
    pub fn ast(&self) -> &DiagramAst {
        &self.ast
    }

    /// Swap AST when kinds match; returns the previous AST. Does not bump `rev`.
    pub fn replace_ast(&mut self, ast: DiagramAst) -> Result<DiagramAst, DiagramAstKindMismatch> {
        let found = ast.kind();
        if found != self.kind {
            return Err(DiagramAstKindMismatch { expected: self.kind, found });
        }

        Ok(std::mem::replace(&mut self.ast, ast))
    }

    /// Like [`Self::replace_ast`] but discards the previous payload.
    pub fn set_ast(&mut self, ast: DiagramAst) -> Result<(), DiagramAstKindMismatch> {
        self.replace_ast(ast).map(|_| ())
    }

    /// Current optimistic-concurrency revision (MCP `base_rev` token).
    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Load or force a revision (e.g. after store restore); prefer [`Self::bump_rev`] for writes.
    pub fn set_rev(&mut self, rev: u64) {
        self.rev = rev;
    }

    /// Default Frigg repository id for unresolved symbol anchors on this diagram.
    pub fn default_symbol_repository_id(&self) -> Option<&str> {
        self.default_symbol_repository_id.as_deref()
    }

    /// Set or clear the diagram-default symbol repository.
    pub fn set_default_symbol_repository_id<T: Into<String>>(&mut self, repository_id: Option<T>) {
        self.default_symbol_repository_id = repository_id.map(Into::into);
    }

    /// Advance OCC rev after a successful mutation; saturates at `u64::MAX`.
    pub fn bump_rev(&mut self) {
        // Saturating: never wrap (would look like a successful older base_rev under OCC).
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

        diagram.set_ast(DiagramAst::Sequence(SequenceAst::default())).expect("set_ast");

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
        let mut diagram =
            Diagram::new(diagram_id, "Example", DiagramAst::Sequence(SequenceAst::default()));

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
