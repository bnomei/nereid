// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Persistence for sessions on disk.
//!
//! The session folder format bundles meta JSON, per-diagram `.mmd` sidecars, walkthrough JSON,
//! cached ASCII exports, and cross-process write locking for TUI/MCP cohabitation.

pub mod session_folder;

pub use session_folder::{
    DiagramMeta, DiagramStableIdMap, DiagramXRef, SessionFolder, SessionMeta, SessionMetaDiagram,
    StoreError, WriteDurability, XRefStatus,
};
