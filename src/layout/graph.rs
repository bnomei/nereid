// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Graph-family layout entry points (flow / class / er).
//!
//! Commit-1 bridges to [`super::layout_flowchart`] via the scene→flowchart AST bridge. Later
//! commits can layout directly from [`crate::render::scene::GraphModel`] metrics (variable box
//! heights, compartments).
//!
//! Layering note: scene types currently live under `render::scene`, so `layout` depends on
//! `render` for the model types. That inversion is temporary; a neutral `scene` module is
//! preferred once paint stops bridging through domain ASTs.

use crate::render::scene::GraphModel;

use super::flowchart::{layout_flowchart, FlowchartLayout, FlowchartLayoutError};

/// Place a graph-family scene.
pub fn layout_graph(model: &GraphModel) -> Result<FlowchartLayout, FlowchartLayoutError> {
    layout_flowchart(&model.to_flowchart_ast())
}
