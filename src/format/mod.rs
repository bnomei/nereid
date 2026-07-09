// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Mermaid-ish parse and export for supported diagram kinds.
//!
//! Bridge between session-folder `.mmd` files and internal sequence/flowchart ASTs. Unsupported
//! Mermaid is rejected with line-oriented errors; export is deterministic for stable diffs.

pub mod mermaid;
