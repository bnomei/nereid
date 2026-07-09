// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! MCP server for agent collaboration over a Nereid session.
//!
//! Tool groups: diagram lifecycle/mutation/replace, walkthroughs, xrefs, graph queries, and
//! co-presence (`attention_*`, `selection_*`, `follow_ai_*`). Stdio (`--mcp`) or streamable HTTP
//! alongside the TUI; durable mode reloads the session folder on each mutation.

mod server;
mod types;

pub use server::NereidMcp;
