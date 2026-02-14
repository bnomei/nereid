// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Layout algorithms for diagrams.
//!
//! This module computes node placement and edge routing for supported diagram kinds.

pub mod flowchart;
pub mod sequence;

pub use flowchart::{
    layout_flowchart, route_flowchart_edges_orthogonal, FlowNodePlacement, FlowchartLayout,
    FlowchartLayoutError, GridPoint,
};
pub use sequence::{layout_sequence, SequenceLayout, SequenceLayoutError};
