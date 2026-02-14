// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Persistence for sessions on disk.
//!
//! The store module reads/writes the session folder format (meta file plus diagram/walkthrough
//! files) used by both the TUI and MCP server.

pub mod session_folder;

pub use session_folder::{
    DiagramMeta, DiagramStableIdMap, DiagramXRef, SessionFolder, SessionMeta, SessionMetaDiagram,
    StoreError, WriteDurability, XRefStatus,
};
