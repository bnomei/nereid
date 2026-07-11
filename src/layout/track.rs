// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Track-family layout entry points (sequence / gantt).
//!
//! Commit-1 bridges to [`super::layout_sequence`]. Gantt will share this family with bar spans
//! and time columns.
//!
//! Layering note: see [`super::graph`] — `layout`→`render::scene` is temporary.

use crate::render::scene::TrackModel;

use super::sequence::{layout_sequence, SequenceLayout, SequenceLayoutError};

/// Place a track-family scene via the temporary sequence-AST bridge.
pub fn layout_track(model: &TrackModel) -> Result<SequenceLayout, SequenceLayoutError> {
    layout_sequence(model.as_sequence_ast())
}
