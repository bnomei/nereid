// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Read-only derived views over diagrams and session xrefs.
//!
//! Flow reachability, sequence search, and multi-diagram route finding for MCP query tools and
//! TUI navigation. Never mutates session content.

pub mod flow;
pub mod sequence;
pub mod session_routes;
