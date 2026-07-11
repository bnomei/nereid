// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Nereid — live diagram session workspace for terminals and agents.
//!
//! Layers a typed session model (sequence/flowchart/class/ER/gantt diagrams, walkthroughs,
//! cross-references, and Frigg symbol anchors) over Mermaid-ish import/export, layout/render, and
//! session folder persistence. Humans use the ratatui TUI; agents use the MCP tool surface. When
//! co-hosted they share revision-gated mutations (`base_rev`), selection, attention, and
//! follow-AI via [`ui::UiState`].

pub mod format;
pub mod layout;
pub mod mcp;
pub mod model;
pub mod ops;
pub mod query;
pub mod render;
pub mod store;
pub mod tui;
pub mod ui;

#[cfg(test)]
mod tests {
    #[test]
    fn sanity() {
        assert_eq!(2 + 2, 4);
    }
}
