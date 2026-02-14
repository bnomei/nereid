// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Core data model and AST types.
//!
//! Sessions contain diagrams (flowchart/sequence) plus walkthroughs and cross-references.

pub mod diagram;
pub(crate) mod fixtures;
pub mod flow_ast;
pub mod ids;
pub mod object_ref;
pub mod seq_ast;
pub mod session;
pub mod walkthrough;
pub mod xref;

pub use diagram::{Diagram, DiagramAst, DiagramAstKindMismatch, DiagramKind};
pub use flow_ast::{FlowEdge, FlowNode, FlowchartAst};
pub use ids::{
    DiagramId, Id, IdError, ObjectId, SessionId, WalkthroughId, WalkthroughNodeId, XRefId,
};
pub use object_ref::{CategoryPath, CategoryPathError, ObjectRef, ParseObjectRefError};
pub use seq_ast::{
    SequenceAst, SequenceMessage, SequenceMessageKind, SequenceNote, SequenceParticipant,
};
pub use session::Session;
pub use walkthrough::{Walkthrough, WalkthroughEdge, WalkthroughNode};
pub use xref::{ParseXRefStatusError, XRef, XRefStatus};
