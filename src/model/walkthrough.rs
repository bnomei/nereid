// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Walkthrough graph: narrative nodes with evidence `ObjectRef`s and directed edges.
//!
//! Separate OCC revision space from diagrams; mutated via walkthrough ops, not diagram ops.
//! Evidence refs point into diagram objects; dangling evidence is the consumer's concern.

use super::ids::{WalkthroughId, WalkthroughNodeId};
use super::object_ref::ObjectRef;

/// Narrative/teaching graph over one or more diagrams in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walkthrough {
    walkthrough_id: WalkthroughId,
    title: String,
    rev: u64,
    nodes: Vec<WalkthroughNode>,
    edges: Vec<WalkthroughEdge>,
    source: Option<String>,
}

impl Walkthrough {
    /// Empty graph with `rev = 0`.
    pub fn new(walkthrough_id: WalkthroughId, title: impl Into<String>) -> Self {
        Self {
            walkthrough_id,
            title: title.into(),
            rev: 0,
            nodes: Vec::new(),
            edges: Vec::new(),
            source: None,
        }
    }

    /// Stable walkthrough id within the session.
    pub fn walkthrough_id(&self) -> &WalkthroughId {
        &self.walkthrough_id
    }

    /// Display title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Replace the display title (does not bump `rev`).
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// OCC revision for walkthrough mutations (`base_rev`).
    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Load or force a revision after store restore.
    pub fn set_rev(&mut self, rev: u64) {
        self.rev = rev;
    }

    /// Advance OCC rev after a successful mutation; saturates at `u64::MAX`.
    pub fn bump_rev(&mut self) {
        // Saturating: never wrap under OCC.
        self.rev = self.rev.saturating_add(1);
    }

    /// Narrative nodes in storage order.
    pub fn nodes(&self) -> &[WalkthroughNode] {
        &self.nodes
    }

    /// Mutable node list.
    pub fn nodes_mut(&mut self) -> &mut Vec<WalkthroughNode> {
        &mut self.nodes
    }

    /// Directed edges between nodes.
    pub fn edges(&self) -> &[WalkthroughEdge] {
        &self.edges
    }

    /// Mutable edge list.
    pub fn edges_mut(&mut self) -> &mut Vec<WalkthroughEdge> {
        &mut self.edges
    }

    /// Optional provenance / generator tag for the walkthrough document.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Set or clear provenance metadata.
    pub fn set_source(&mut self, source: Option<String>) {
        self.source = source;
    }
}

/// Narrative step with optional markdown body and evidence `ObjectRef`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkthroughNode {
    node_id: WalkthroughNodeId,
    title: String,
    body_md: Option<String>,
    refs: Vec<ObjectRef>,
    tags: Vec<String>,
    status: Option<String>,
}

impl WalkthroughNode {
    /// Node with empty body, refs, tags, and status.
    pub fn new(node_id: WalkthroughNodeId, title: impl Into<String>) -> Self {
        Self {
            node_id,
            title: title.into(),
            body_md: None,
            refs: Vec::new(),
            tags: Vec::new(),
            status: None,
        }
    }

    /// Stable node id within this walkthrough.
    pub fn node_id(&self) -> &WalkthroughNodeId {
        &self.node_id
    }

    /// Step title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Replace the step title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Optional markdown body for the step.
    pub fn body_md(&self) -> Option<&str> {
        self.body_md.as_deref()
    }

    /// Set or clear the markdown body.
    pub fn set_body_md(&mut self, body_md: Option<String>) {
        self.body_md = body_md;
    }

    /// Evidence links into diagram objects (evidence-first walkthroughs).
    pub fn refs(&self) -> &[ObjectRef] {
        &self.refs
    }

    /// Mutable evidence list.
    pub fn refs_mut(&mut self) -> &mut Vec<ObjectRef> {
        &mut self.refs
    }

    /// Free-form tags for filtering/grouping.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Mutable tag list.
    pub fn tags_mut(&mut self) -> &mut Vec<String> {
        &mut self.tags
    }

    /// Optional free-form status (not the same as [`crate::model::XRefStatus`]).
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// Set or clear free-form status.
    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }
}

/// Directed edge between two walkthrough nodes (kind is free-form).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkthroughEdge {
    from_node_id: WalkthroughNodeId,
    to_node_id: WalkthroughNodeId,
    kind: String,
    label: Option<String>,
}

impl WalkthroughEdge {
    /// Edge with no label.
    pub fn new(
        from_node_id: WalkthroughNodeId,
        to_node_id: WalkthroughNodeId,
        kind: impl Into<String>,
    ) -> Self {
        Self { from_node_id, to_node_id, kind: kind.into(), label: None }
    }

    /// Source node id.
    pub fn from_node_id(&self) -> &WalkthroughNodeId {
        &self.from_node_id
    }

    /// Target node id.
    pub fn to_node_id(&self) -> &WalkthroughNodeId {
        &self.to_node_id
    }

    /// Free-form edge kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Optional display label.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Set or clear the display label.
    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label;
    }
}
