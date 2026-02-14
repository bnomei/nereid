// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Shared UI state for cross-component coordination.
//!
//! This lightweight state is used to propagate selection context between the interactive TUI and
//! programmatic integrations (MCP).

use crate::model::{DiagramId, ObjectRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    rev: u64,
    human_active_diagram_id: Option<DiagramId>,
    human_active_object_ref: Option<ObjectRef>,
    follow_ai: bool,
    session_rev: u64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            rev: 0,
            human_active_diagram_id: None,
            human_active_object_ref: None,
            follow_ai: true,
            session_rev: 0,
        }
    }
}

impl UiState {
    pub fn rev(&self) -> u64 {
        self.rev
    }

    pub fn human_active_diagram_id(&self) -> Option<&DiagramId> {
        self.human_active_diagram_id.as_ref()
    }

    pub fn human_active_object_ref(&self) -> Option<&ObjectRef> {
        self.human_active_object_ref.as_ref()
    }

    pub fn follow_ai(&self) -> bool {
        self.follow_ai
    }

    pub fn session_rev(&self) -> u64 {
        self.session_rev
    }

    pub fn set_human_selection(
        &mut self,
        active_diagram_id: Option<DiagramId>,
        active_object_ref: Option<ObjectRef>,
    ) {
        let active_diagram_id = active_object_ref
            .as_ref()
            .map(|object_ref| object_ref.diagram_id().clone())
            .or(active_diagram_id);

        if self.human_active_diagram_id == active_diagram_id
            && self.human_active_object_ref == active_object_ref
        {
            return;
        }

        self.human_active_diagram_id = active_diagram_id;
        self.human_active_object_ref = active_object_ref;
        self.rev = self.rev.wrapping_add(1);
    }

    pub fn set_follow_ai(&mut self, follow_ai: bool) {
        if self.follow_ai == follow_ai {
            return;
        }
        self.follow_ai = follow_ai;
        self.rev = self.rev.wrapping_add(1);
    }

    pub fn bump_session_rev(&mut self) {
        self.session_rev = self.session_rev.wrapping_add(1);
        self.rev = self.rev.wrapping_add(1);
    }
}
