// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Diagram format parsing and export.
//!
//! Converts between on-disk Mermaid sidecars and the internal flowchart/sequence ASTs used by
//! layout, render, and mutation layers.

pub mod mermaid;
