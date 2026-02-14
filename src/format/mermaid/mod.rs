// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Mermaid-ish parsing and exporting for supported diagram kinds.

pub mod flowchart;
mod ident;
pub mod sequence;

pub use sequence::{
    export_sequence_diagram, parse_sequence_diagram, MermaidSequenceExportError,
    MermaidSequenceParseError,
};

pub use flowchart::{
    export_flowchart, parse_flowchart, MermaidFlowchartExportError, MermaidFlowchartParseError,
};
