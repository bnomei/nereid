// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Session domain model and diagram ASTs.
//!
//! Owns the in-memory truth for **sessions**: typed diagram ASTs, walkthrough narratives, xrefs,
//! and multi-object selection. Stable ids (`ObjectId`, `ObjectRef`) and revision (`base_rev`/OCC)
//! live here and in store sidecars; Mermaid text and Unicode renders are projections, not sources
//! of identity.

pub mod class_ast;
pub mod diagram;
pub mod er_ast;
pub(crate) mod fixtures;
pub mod flow_ast;
pub mod gantt_ast;
pub mod ids;
pub mod object_ref;
pub mod seq_ast;
pub mod session;
pub mod symbol;
pub mod walkthrough;
pub mod xref;

pub use class_ast::{ClassAst, ClassNode, ClassRelation, ClassRelationKind};
pub use diagram::{Diagram, DiagramAst, DiagramAstKindMismatch, DiagramKind};
pub use er_ast::{ErAst, ErCardinality, ErEntity, ErRelationship, ErStroke};
pub use flow_ast::{FlowEdge, FlowNode, FlowchartAst};
pub use gantt_ast::{GanttAst, GanttSection, GanttTask, GanttTaskStart};
pub use ids::{
    DiagramId, Id, IdError, ObjectId, SessionId, WalkthroughId, WalkthroughNodeId, XRefId,
};
pub use object_ref::{CategoryPath, CategoryPathError, ObjectRef, ParseObjectRefError};
pub use seq_ast::{
    SequenceAst, SequenceMessage, SequenceMessageKind, SequenceNote, SequenceParticipant,
};
pub use session::Session;
pub use symbol::{SymbolAnchor, SymbolAnchorError};
pub use walkthrough::{Walkthrough, WalkthroughEdge, WalkthroughNode};
pub use xref::{ParseXRefStatusError, XRef, XRefStatus};
