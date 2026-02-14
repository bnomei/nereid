// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Read-only queries over sessions and diagrams.
//!
//! Queries provide derived views (e.g. routes/relationships) that power the UI and MCP tools.

pub mod flow;
pub mod sequence;
pub mod session_routes;
