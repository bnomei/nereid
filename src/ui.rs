// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Shared UI attention/follow-AI state between TUI and MCP.
//!
//! Carries human and agent focus refs and follow-AI so MCP tools can read or steer the live TUI
//! without owning terminal I/O.

use crate::model::{DiagramId, ObjectRef};

/// Cross-process UI snapshot: human selection, follow-AI flag, and revision counters.
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
    /// Monotonic UI revision; bumps on selection, follow-AI, or session changes.
    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Diagram the human is viewing (`human_active`), if any.
    pub fn human_active_diagram_id(&self) -> Option<&DiagramId> {
        self.human_active_diagram_id.as_ref()
    }

    /// Object currently under human attention (single spotlight).
    pub fn human_active_object_ref(&self) -> Option<&ObjectRef> {
        self.human_active_object_ref.as_ref()
    }

    /// When true, TUI camera follows agent highlight / attention updates.
    pub fn follow_ai(&self) -> bool {
        self.follow_ai
    }

    /// Session content revision observed by the UI (MCP mutations bump this).
    pub fn session_rev(&self) -> u64 {
        self.session_rev
    }

    /// Publish human attention (diagram + optional object). Object ref wins for diagram id.
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

    /// Enable/disable follow-AI camera steering from MCP attention tools.
    pub fn set_follow_ai(&mut self, follow_ai: bool) {
        if self.follow_ai == follow_ai {
            return;
        }
        self.follow_ai = follow_ai;
        self.rev = self.rev.wrapping_add(1);
    }

    /// Signal that session folder content changed (TUI should reload).
    pub fn bump_session_rev(&mut self) {
        self.session_rev = self.session_rev.wrapping_add(1);
        self.rev = self.rev.wrapping_add(1);
    }
}
