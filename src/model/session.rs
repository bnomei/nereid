// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::{BTreeMap, BTreeSet};

use super::diagram::{Diagram, DiagramAst};
use super::ids::{DiagramId, SessionId, WalkthroughId, XRefId};
use super::object_ref::ObjectRef;
use super::walkthrough::Walkthrough;
use super::xref::XRef;

/// The top-level container the TUI runs against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    session_id: SessionId,
    diagrams: BTreeMap<DiagramId, Diagram>,
    walkthroughs: BTreeMap<WalkthroughId, Walkthrough>,
    xrefs: BTreeMap<XRefId, XRef>,
    active_diagram_id: Option<DiagramId>,
    active_walkthrough_id: Option<WalkthroughId>,
    selected_object_refs: BTreeSet<ObjectRef>,
}

impl Session {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            diagrams: BTreeMap::new(),
            walkthroughs: BTreeMap::new(),
            xrefs: BTreeMap::new(),
            active_diagram_id: None,
            active_walkthrough_id: None,
            selected_object_refs: BTreeSet::new(),
        }
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn diagrams(&self) -> &BTreeMap<DiagramId, Diagram> {
        &self.diagrams
    }

    pub fn diagrams_mut(&mut self) -> &mut BTreeMap<DiagramId, Diagram> {
        &mut self.diagrams
    }

    pub fn walkthroughs(&self) -> &BTreeMap<WalkthroughId, Walkthrough> {
        &self.walkthroughs
    }

    pub fn walkthroughs_mut(&mut self) -> &mut BTreeMap<WalkthroughId, Walkthrough> {
        &mut self.walkthroughs
    }

    pub fn xrefs(&self) -> &BTreeMap<XRefId, XRef> {
        &self.xrefs
    }

    pub fn xrefs_mut(&mut self) -> &mut BTreeMap<XRefId, XRef> {
        &mut self.xrefs
    }

    pub fn object_ref_exists(&self, object_ref: &ObjectRef) -> bool {
        let Some(diagram) = self.diagrams.get(object_ref.diagram_id()) else {
            return false;
        };

        let segments = object_ref.category().segments();
        let object_id = object_ref.object_id();

        match (diagram.ast(), segments) {
            (DiagramAst::Sequence(ast), [left, right])
                if left == "seq" && right == "participant" =>
            {
                ast.participants().contains_key(object_id)
            }
            (DiagramAst::Sequence(ast), [left, right]) if left == "seq" && right == "message" => {
                ast.messages().iter().any(|m| m.message_id() == object_id)
            }
            (DiagramAst::Sequence(ast), [left, right]) if left == "seq" && right == "block" => {
                ast.find_block(object_id).is_some()
            }
            (DiagramAst::Sequence(ast), [left, right]) if left == "seq" && right == "section" => {
                ast.find_section(object_id).is_some()
            }
            (DiagramAst::Flowchart(ast), [left, right]) if left == "flow" && right == "node" => {
                ast.nodes().contains_key(object_id)
            }
            (DiagramAst::Flowchart(ast), [left, right]) if left == "flow" && right == "edge" => {
                ast.edges().contains_key(object_id)
            }
            _ => false,
        }
    }

    pub fn object_ref_is_missing(&self, object_ref: &ObjectRef) -> bool {
        !self.object_ref_exists(object_ref)
    }

    pub fn active_diagram_id(&self) -> Option<&DiagramId> {
        self.active_diagram_id.as_ref()
    }

    pub fn set_active_diagram_id(&mut self, diagram_id: Option<DiagramId>) {
        self.active_diagram_id = diagram_id;
    }

    pub fn active_walkthrough_id(&self) -> Option<&WalkthroughId> {
        self.active_walkthrough_id.as_ref()
    }

    pub fn set_active_walkthrough_id(&mut self, walkthrough_id: Option<WalkthroughId>) {
        self.active_walkthrough_id = walkthrough_id;
    }

    pub fn selected_object_refs(&self) -> &BTreeSet<ObjectRef> {
        &self.selected_object_refs
    }

    pub fn selected_object_refs_mut(&mut self) -> &mut BTreeSet<ObjectRef> {
        &mut self.selected_object_refs
    }

    pub fn set_selected_object_refs(&mut self, selected_object_refs: BTreeSet<ObjectRef>) {
        self.selected_object_refs = selected_object_refs;
    }
}
