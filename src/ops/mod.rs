// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Mutation operations for diagrams/sessions.
//!
//! Operations are applied with optimistic concurrency (revision checks) and produce a minimal
//! delta that the UI can use to refresh derived state.

use std::collections::HashSet;
use std::fmt;

use crate::format::mermaid::flowchart::MermaidIdentError;
use crate::model::{
    CategoryPath, Diagram, DiagramAst, DiagramId, DiagramKind, FlowEdge, FlowNode, FlowchartAst,
};
use crate::model::{ObjectId, ObjectRef, SequenceAst, SequenceMessage, SequenceMessageKind};
use crate::model::{SequenceParticipant, XRefId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Seq(SeqOp),
    Flow(FlowOp),
    XRef(XRefOp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeqOp {
    AddParticipant {
        participant_id: ObjectId,
        mermaid_name: String,
    },
    UpdateParticipant {
        participant_id: ObjectId,
        patch: SeqParticipantPatch,
    },
    SetParticipantNote {
        participant_id: ObjectId,
        note: Option<String>,
    },
    RemoveParticipant {
        participant_id: ObjectId,
    },
    AddMessage {
        message_id: ObjectId,
        from_participant_id: ObjectId,
        to_participant_id: ObjectId,
        kind: SequenceMessageKind,
        arrow: Option<String>,
        text: String,
        order_key: i64,
    },
    UpdateMessage {
        message_id: ObjectId,
        patch: SeqMessagePatch,
    },
    RemoveMessage {
        message_id: ObjectId,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeqParticipantPatch {
    pub mermaid_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeqMessagePatch {
    pub from_participant_id: Option<ObjectId>,
    pub to_participant_id: Option<ObjectId>,
    pub kind: Option<SequenceMessageKind>,
    pub arrow: Option<String>,
    pub text: Option<String>,
    pub order_key: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowOp {
    AddNode {
        node_id: ObjectId,
        label: String,
        shape: Option<String>,
    },
    UpdateNode {
        node_id: ObjectId,
        patch: FlowNodePatch,
    },
    SetNodeMermaidId {
        node_id: ObjectId,
        mermaid_id: Option<String>,
    },
    SetNodeNote {
        node_id: ObjectId,
        note: Option<String>,
    },
    RemoveNode {
        node_id: ObjectId,
    },
    AddEdge {
        edge_id: ObjectId,
        from_node_id: ObjectId,
        to_node_id: ObjectId,
        label: Option<String>,
        connector: Option<String>,
        style: Option<String>,
    },
    UpdateEdge {
        edge_id: ObjectId,
        patch: FlowEdgePatch,
    },
    RemoveEdge {
        edge_id: ObjectId,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowNodePatch {
    pub label: Option<String>,
    pub shape: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowEdgePatch {
    pub from_node_id: Option<ObjectId>,
    pub to_node_id: Option<ObjectId>,
    pub label: Option<String>,
    pub connector: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XRefOp {
    Add { xref_id: XRefId, from: ObjectRef, to: ObjectRef, kind: String, label: Option<String> },
    Remove { xref_id: XRefId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub new_rev: u64,
    pub applied: usize,
    pub delta: Delta,
}

/// Minimal delta describing which objects changed as the result of applying ops.
///
/// This is intentionally coarse: it reports only added/removed/updated `ObjectRef`s.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Delta {
    pub added: Vec<ObjectRef>,
    pub removed: Vec<ObjectRef>,
    pub updated: Vec<ObjectRef>,
}

#[derive(Debug, Default)]
struct DeltaBuilder {
    added: HashSet<ObjectRef>,
    removed: HashSet<ObjectRef>,
    updated: HashSet<ObjectRef>,
}

impl DeltaBuilder {
    fn record_added(&mut self, object_ref: ObjectRef) {
        self.removed.remove(&object_ref);
        self.updated.remove(&object_ref);
        self.added.insert(object_ref);
    }

    fn record_removed(&mut self, object_ref: ObjectRef) {
        self.added.remove(&object_ref);
        self.updated.remove(&object_ref);
        self.removed.insert(object_ref);
    }

    fn record_updated(&mut self, object_ref: ObjectRef) {
        if self.added.contains(&object_ref) || self.removed.contains(&object_ref) {
            return;
        }
        self.updated.insert(object_ref);
    }

    fn finish(self) -> Delta {
        let mut added = self.added.into_iter().collect::<Vec<_>>();
        let mut removed = self.removed.into_iter().collect::<Vec<_>>();
        let mut updated = self.updated.into_iter().collect::<Vec<_>>();

        sort_object_refs(&mut added);
        sort_object_refs(&mut removed);
        sort_object_refs(&mut updated);

        Delta { added, removed, updated }
    }
}

fn sort_object_refs(refs: &mut [ObjectRef]) {
    refs.sort_by(|a, b| {
        a.diagram_id()
            .cmp(b.diagram_id())
            .then_with(|| a.category().cmp(b.category()))
            .then_with(|| a.object_id().cmp(b.object_id()))
    });
}

pub fn apply_ops(
    diagram: &mut Diagram,
    base_rev: u64,
    ops: &[Op],
) -> Result<ApplyResult, ApplyError> {
    let current_rev = diagram.rev();
    if base_rev != current_rev {
        return Err(ApplyError::Conflict { base_rev, current_rev });
    }

    if ops.is_empty() {
        return Ok(ApplyResult { new_rev: current_rev, applied: 0, delta: Delta::default() });
    }

    let mut new_ast = diagram.ast().clone();
    let diagram_id = diagram.diagram_id().clone();
    let mut delta = DeltaBuilder::default();

    for op in ops {
        match op {
            Op::Seq(seq_op) => {
                let DiagramAst::Sequence(ast) = &mut new_ast else {
                    return Err(ApplyError::KindMismatch {
                        diagram_kind: diagram.kind(),
                        op_kind: OpKind::Seq,
                    });
                };
                apply_seq_op(&diagram_id, ast, seq_op, &mut delta)?;
            }
            Op::Flow(flow_op) => {
                let DiagramAst::Flowchart(ast) = &mut new_ast else {
                    return Err(ApplyError::KindMismatch {
                        diagram_kind: diagram.kind(),
                        op_kind: OpKind::Flow,
                    });
                };
                apply_flow_op(&diagram_id, ast, flow_op, &mut delta)?;
            }
            Op::XRef(_) => {
                return Err(ApplyError::UnsupportedOp { op_kind: OpKind::XRef });
            }
        }
    }

    diagram.set_ast(new_ast).map_err(|mismatch| {
        let op_kind = match mismatch.found() {
            DiagramKind::Sequence => OpKind::Seq,
            DiagramKind::Flowchart => OpKind::Flow,
        };
        ApplyError::KindMismatch { diagram_kind: mismatch.expected(), op_kind }
    })?;
    diagram.bump_rev();
    let new_rev = diagram.rev();

    Ok(ApplyResult { new_rev, applied: ops.len(), delta: delta.finish() })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Seq,
    Flow,
    XRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    SeqParticipant,
    SeqMessage,
    FlowNode,
    FlowEdge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    Conflict { base_rev: u64, current_rev: u64 },
    KindMismatch { diagram_kind: DiagramKind, op_kind: OpKind },
    UnsupportedOp { op_kind: OpKind },
    AlreadyExists { kind: ObjectKind, object_id: ObjectId },
    NotFound { kind: ObjectKind, object_id: ObjectId },
    MissingFlowNode { node_id: ObjectId },
    InvalidFlowNodeMermaidId { mermaid_id: String, reason: MermaidIdentError },
    DuplicateFlowNodeMermaidId { mermaid_id: String, node_id: ObjectId },
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { base_rev, current_rev } => {
                write!(f, "stale base_rev (base_rev={base_rev}, current_rev={current_rev})")
            }
            Self::KindMismatch { diagram_kind, op_kind } => {
                write!(f, "op kind mismatch (diagram_kind={diagram_kind:?}, op_kind={op_kind:?})")
            }
            Self::UnsupportedOp { op_kind } => write!(f, "unsupported op kind ({op_kind:?})"),
            Self::AlreadyExists { kind, object_id } => {
                write!(f, "object already exists ({kind:?}, id={object_id})")
            }
            Self::NotFound { kind, object_id } => {
                write!(f, "object not found ({kind:?}, id={object_id})")
            }
            Self::MissingFlowNode { node_id } => write!(f, "flow node not found (id={node_id})"),
            Self::InvalidFlowNodeMermaidId { mermaid_id, reason } => {
                write!(f, "invalid flow node Mermaid id '{mermaid_id}': {reason}")
            }
            Self::DuplicateFlowNodeMermaidId { mermaid_id, node_id } => {
                write!(f, "flow node Mermaid id '{mermaid_id}' is already used by node {node_id}")
            }
        }
    }
}

impl std::error::Error for ApplyError {}

// Extracted op-application implementation for sequence/flow mutations.
include!("ops_impl.rs");

#[cfg(test)]
mod tests;
