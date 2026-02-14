// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Nereid — Rust Diagram TUI (AST + MCP + Walkthroughs).
//!
//! This crate starts as a single-crate layout per `specs/01-diagram-tui-rust/design.md`.

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
