// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Session-folder persistence and identity reconciliation.
//!
//! On-disk format: `nereid-session.meta.json`, `diagrams/*.mmd` + `*.meta.json` sidecars
//! (stable ids, notes, symbols, sequence_blocks), walkthrough JSON, best-effort text exports,
//! and a write lock for TUI/MCP cohabitation. Load re-parses Mermaid then reconciles ids from
//! sidecars so xrefs survive rewrites.

pub mod session_folder;

pub use session_folder::{
    replace_diagram_from_mermaid, DiagramMermaidReplaceError, DiagramMermaidReplaceResult,
    DiagramMeta, DiagramStableIdMap, DiagramXRef, SessionFolder, SessionMeta, SessionMetaDiagram,
    StoreError, WriteDurability, XRefStatus,
};
