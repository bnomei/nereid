// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Deterministic diagram layout and edge routing.
//!
//! Places sequence participants/message rows and flowchart layers on integer grids used by
//! Unicode renderers. Layout is pure: no I/O; failures are unrouteable or incomplete graphs.

pub mod flowchart;
pub mod graph;
pub mod sequence;
pub mod track;

pub use flowchart::{
    layout_flowchart, route_flowchart_edges_orthogonal, FlowNodePlacement, FlowchartLayout,
    FlowchartLayoutError, GridPoint,
};
pub use graph::layout_graph;
pub use sequence::{layout_sequence, SequenceLayout, SequenceLayoutError};
pub use track::layout_track;
