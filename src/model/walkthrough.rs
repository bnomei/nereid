// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::ids::{WalkthroughId, WalkthroughNodeId};
use super::object_ref::ObjectRef;

/// A narrative/teaching layer over one or more diagrams in a session.
///
/// See `docs/protocol-01.md` §2.5.
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

    pub fn walkthrough_id(&self) -> &WalkthroughId {
        &self.walkthrough_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
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

    pub fn nodes(&self) -> &[WalkthroughNode] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<WalkthroughNode> {
        &mut self.nodes
    }

    pub fn edges(&self) -> &[WalkthroughEdge] {
        &self.edges
    }

    pub fn edges_mut(&mut self) -> &mut Vec<WalkthroughEdge> {
        &mut self.edges
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn set_source(&mut self, source: Option<String>) {
        self.source = source;
    }
}

/// A single narrative node in a [`Walkthrough`].
///
/// See `docs/protocol-01.md` §2.5 ("Evidence-first": `refs`).
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

    pub fn node_id(&self) -> &WalkthroughNodeId {
        &self.node_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn body_md(&self) -> Option<&str> {
        self.body_md.as_deref()
    }

    pub fn set_body_md(&mut self, body_md: Option<String>) {
        self.body_md = body_md;
    }

    pub fn refs(&self) -> &[ObjectRef] {
        &self.refs
    }

    pub fn refs_mut(&mut self) -> &mut Vec<ObjectRef> {
        &mut self.refs
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn tags_mut(&mut self) -> &mut Vec<String> {
        &mut self.tags
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }
}

/// Directed edge between two walkthrough nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkthroughEdge {
    from_node_id: WalkthroughNodeId,
    to_node_id: WalkthroughNodeId,
    kind: String,
    label: Option<String>,
}

impl WalkthroughEdge {
    pub fn new(
        from_node_id: WalkthroughNodeId,
        to_node_id: WalkthroughNodeId,
        kind: impl Into<String>,
    ) -> Self {
        Self { from_node_id, to_node_id, kind: kind.into(), label: None }
    }

    pub fn from_node_id(&self) -> &WalkthroughNodeId {
        &self.from_node_id
    }

    pub fn to_node_id(&self) -> &WalkthroughNodeId {
        &self.to_node_id
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label;
    }
}
