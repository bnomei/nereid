// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Mermaid-ish parsing and exporting for supported diagram kinds.
//!
//! Each submodule owns one diagram kind's limited subset: `parse_*` builds a model AST with
//! positional stable ids; `export_*` emits canonical `.mmd`. Shared identifier rules live in
//! `ident` (not re-exported beyond error types used by public parsers).

pub mod class;
pub mod er;
pub mod flowchart;
pub mod gantt;
mod ident;
pub mod sequence;

pub use class::{
    export_class_diagram, parse_class_diagram, MermaidClassExportError, MermaidClassParseError,
};
pub use er::{export_er_diagram, parse_er_diagram, MermaidErExportError, MermaidErParseError};
pub use gantt::{
    export_gantt_diagram, parse_gantt_diagram, MermaidGanttExportError, MermaidGanttParseError,
};
pub use sequence::{
    export_sequence_diagram, parse_sequence_diagram, MermaidSequenceExportError,
    MermaidSequenceParseError,
};

pub use flowchart::{
    export_flowchart, parse_flowchart, MermaidFlowchartExportError, MermaidFlowchartParseError,
};
