// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Model Context Protocol (MCP) server surface.
//!
//! The MCP layer provides a programmatic interface for inspecting and mutating sessions.

mod server;
mod types;

pub use server::NereidMcp;
