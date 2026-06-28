// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Model Context Protocol (MCP) server surface.
//!
//! Exposes diagram, walkthrough, xref, query, and collaboration tools over stdio or HTTP,
//! sharing session revisions and UI attention state with the TUI when co-hosted.

mod server;
mod types;

pub use server::NereidMcp;
