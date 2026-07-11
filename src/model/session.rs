// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! In-memory session aggregate shared by TUI and MCP.
//!
//! Live working set: diagrams (with OCC rev), walkthroughs, xrefs, active targets, and multi-object
//! selection. Disk durability is the store layer; this type never reads or writes paths itself.

use std::collections::{BTreeMap, BTreeSet};

use super::diagram::{Diagram, DiagramAst};
use super::ids::{DiagramId, SessionId, WalkthroughId, XRefId};
use super::object_ref::ObjectRef;
use super::walkthrough::Walkthrough;
use super::xref::XRef;

/// Live workspace: diagrams, walkthroughs, xrefs, active targets, and selection.
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
    /// Create an empty session with the given id (no diagrams yet).
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

    /// Stable session container id (folder / MCP scope).
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// All diagrams keyed by stable id.
    pub fn diagrams(&self) -> &BTreeMap<DiagramId, Diagram> {
        &self.diagrams
    }

    /// Mutable diagram map for insert/remove/replace by tools and store load.
    pub fn diagrams_mut(&mut self) -> &mut BTreeMap<DiagramId, Diagram> {
        &mut self.diagrams
    }

    /// Walkthrough graphs in this session.
    pub fn walkthroughs(&self) -> &BTreeMap<WalkthroughId, Walkthrough> {
        &self.walkthroughs
    }

    /// Mutable walkthrough map.
    pub fn walkthroughs_mut(&mut self) -> &mut BTreeMap<WalkthroughId, Walkthrough> {
        &mut self.walkthroughs
    }

    /// Cross-diagram xrefs keyed by stable id.
    pub fn xrefs(&self) -> &BTreeMap<XRefId, XRef> {
        &self.xrefs
    }

    /// Mutable xref map.
    pub fn xrefs_mut(&mut self) -> &mut BTreeMap<XRefId, XRef> {
        &mut self.xrefs
    }

    /// Whether `object_ref` resolves to a live object in its diagram-specific category.
    ///
    /// Category segments must match known kind paths (`seq/message`, `flow/node`, `gantt/lane`, …).
    /// Sidecar-only notes on classes/entities/participants are not separate categories; sequence
    /// `SequenceNote`s are also not checked here (no `seq/note` path).
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
            (DiagramAst::Class(ast), [left, right]) if left == "class" && right == "class" => {
                ast.classes().contains_key(object_id)
            }
            (DiagramAst::Class(ast), [left, right]) if left == "class" && right == "relation" => {
                ast.relations().contains_key(object_id)
            }
            (DiagramAst::Er(ast), [left, right]) if left == "er" && right == "entity" => {
                ast.entities().contains_key(object_id)
            }
            (DiagramAst::Er(ast), [left, right]) if left == "er" && right == "relationship" => {
                ast.relationships().contains_key(object_id)
            }
            (DiagramAst::Gantt(ast), [left, right]) if left == "gantt" && right == "task" => {
                ast.tasks().contains_key(object_id)
            }
            (DiagramAst::Gantt(ast), [left, right]) if left == "gantt" && right == "section" => {
                ast.sections().iter().any(|section| section.section_id() == object_id)
            }
            (DiagramAst::Gantt(ast), [left, right]) if left == "gantt" && right == "lane" => {
                ast.lanes().contains_key(object_id)
            }
            _ => false,
        }
    }

    /// Inverse of [`Self::object_ref_exists`] for dangling xref status updates.
    pub fn object_ref_is_missing(&self, object_ref: &ObjectRef) -> bool {
        !self.object_ref_exists(object_ref)
    }

    /// Session-default diagram for tools/UI that omit `diagram_id`.
    pub fn active_diagram_id(&self) -> Option<&DiagramId> {
        self.active_diagram_id.as_ref()
    }

    /// Set or clear the session-default diagram (not validated against `diagrams`).
    pub fn set_active_diagram_id(&mut self, diagram_id: Option<DiagramId>) {
        self.active_diagram_id = diagram_id;
    }

    /// Session-default walkthrough for narrative tools.
    pub fn active_walkthrough_id(&self) -> Option<&WalkthroughId> {
        self.active_walkthrough_id.as_ref()
    }

    /// Set or clear the active walkthrough id.
    pub fn set_active_walkthrough_id(&mut self, walkthrough_id: Option<WalkthroughId>) {
        self.active_walkthrough_id = walkthrough_id;
    }

    /// Multi-object working set shared by TUI selection and MCP `selection_*` tools.
    pub fn selected_object_refs(&self) -> &BTreeSet<ObjectRef> {
        &self.selected_object_refs
    }

    /// Mutable selection set for incremental add/remove.
    pub fn selected_object_refs_mut(&mut self) -> &mut BTreeSet<ObjectRef> {
        &mut self.selected_object_refs
    }

    /// Replace the entire selection set atomically.
    pub fn set_selected_object_refs(&mut self, selected_object_refs: BTreeSet<ObjectRef>) {
        self.selected_object_refs = selected_object_refs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::mermaid::parse_gantt_diagram;
    use crate::model::{CategoryPath, Diagram, DiagramId, ObjectId, SessionId};

    fn object_ref(diagram_id: &DiagramId, category: &str, object_id: &str) -> ObjectRef {
        ObjectRef::new(
            diagram_id.clone(),
            CategoryPath::new(vec!["gantt".to_owned(), category.to_owned()]).unwrap(),
            ObjectId::new(object_id).unwrap(),
        )
    }

    #[test]
    fn gantt_section_and_only_rendered_lanes_are_live_refs() {
        let ast = parse_gantt_diagram(
            "gantt\ndateFormat YYYY-MM-DD\nsection Build\nA :a, 2026-01-01, 14d\n",
        )
        .unwrap();
        let section_id = ast.sections()[0].section_id().clone();
        let diagram_id = DiagramId::new("d-gantt").unwrap();
        let mut session = Session::new(SessionId::new("s:gantt").unwrap());
        session.diagrams_mut().insert(
            diagram_id.clone(),
            Diagram::new(diagram_id.clone(), "Gantt", DiagramAst::Gantt(ast)),
        );

        assert!(session.object_ref_exists(&object_ref(
            &diagram_id,
            "section",
            section_id.as_str()
        )));
        assert!(session.object_ref_exists(&object_ref(&diagram_id, "lane", "lane:2026-01-01")));
        assert!(session.object_ref_exists(&object_ref(&diagram_id, "lane", "lane:2026-01-08")));
        assert!(!session.object_ref_exists(&object_ref(&diagram_id, "lane", "lane:2026-01-02")));
        assert!(!session.object_ref_exists(&object_ref(&diagram_id, "lane", "lane:9999")));
    }
}
