// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Nereid — diagram session editor with terminal UI and MCP integration.
//!
//! The crate layers a typed session model (diagrams, walkthroughs, cross-references) over
//! Mermaid-ish import/export, layout/render pipelines, and folder persistence. The TUI and
//! MCP server share session state, selection, and revision-tracked mutations.

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
