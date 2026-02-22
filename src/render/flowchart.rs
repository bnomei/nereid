// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::layout::flowchart::route_flowchart_edges_orthogonal_key_order;
use crate::layout::{FlowchartLayout, GridPoint};
use crate::model::flow_ast::FlowchartAst;
use crate::model::ids::{DiagramId, ObjectId};
use crate::model::{CategoryPath, ObjectRef};

use super::text::{canvas_to_string_trimmed, text_len, truncate_with_ellipsis};
use super::RenderOptions;
use super::{
    clamp_highlight_index_to_text, AnnotatedRender, Canvas, CanvasError, HighlightIndex, LineSpan,
};

const BOX_HEIGHT_NO_NOTES: usize = 3;
const BOX_HEIGHT_WITH_NOTES: usize = 4;
const MIN_COL_GAP: usize = 1;
const ROW_GAP: usize = 2;
const MIN_BOX_INNER_WIDTH: usize = 3;
const OBJECT_LABEL_PREFIX: &str = "▴ ";
const LANE_MIN_X_CLEARANCE: usize = 2;
const STUB_ROW_KEEPOUT_RADIUS: usize = 1;
// Keep global widening effectively disabled; per-gap lane assignment handles local widening.
const MAX_GLOBAL_CLEARANCE_WIDEN_STEPS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectorDrawPass {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayerMetrics {
    x0: usize,
    x1: usize,
    inner_width: usize,
    total_width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NodeRender {
    layer: usize,
    index_in_layer: usize,
    box_x0: usize,
    box_x1: usize,
    box_y0: usize,
    box_y1: usize,
}

impl NodeRender {
    fn mid_y(self) -> usize {
        self.box_y0 + 1
    }
}

#[derive(Debug, Clone)]
struct FlowchartRenderPlan {
    options: RenderOptions,
    box_height: usize,
    layer_metrics: Vec<LayerMetrics>,
    gap_widths: Vec<usize>,
    node_renders: BTreeMap<ObjectId, NodeRender>,
    routes: Vec<Vec<GridPoint>>,
    edge_gap_lanes: Vec<Vec<Option<usize>>>,
    width: usize,
    height: usize,
}

impl FlowchartRenderPlan {
    fn build(
        ast: &FlowchartAst,
        layout: &FlowchartLayout,
        options: RenderOptions,
    ) -> Result<Self, FlowchartRenderError> {
        let box_height = flow_box_height(options);
        let routes = route_flowchart_edges_orthogonal_key_order(ast, layout);
        let min_col_gap = MIN_COL_GAP.saturating_add(options.flowchart_extra_col_gap);
        let attempt_count = MAX_GLOBAL_CLEARANCE_WIDEN_STEPS;

        let mut attempt_min_col_gap = min_col_gap;
        let mut fallback = None::<(
            Vec<LayerMetrics>,
            Vec<usize>,
            BTreeMap<ObjectId, NodeRender>,
            Vec<Vec<Option<usize>>>,
            usize,
            usize,
        )>;

        for _ in 0..attempt_count {
            let initial_gap_widths =
                vec![attempt_min_col_gap; layout.layers().len().saturating_sub(1)];
            let initial_layer_metrics = layer_metrics(ast, layout, &initial_gap_widths, options)?;
            let (initial_node_renders, base_height) =
                node_renders(layout, &initial_layer_metrics, box_height)?;

            let (edge_gap_lanes, gap_widths) = assign_edge_gap_lanes(
                ast,
                &initial_node_renders,
                initial_layer_metrics.len(),
                &routes,
                box_height,
                attempt_min_col_gap,
            );

            let layer_metrics = layer_metrics(ast, layout, &gap_widths, options)?;
            let (node_renders, _base_height) = node_renders(layout, &layer_metrics, box_height)?;

            let width = layer_metrics.last().map(|layer| layer.x1 + 1).unwrap_or(1);
            let height = routed_height(base_height, &routes, box_height);

            let has_touch = has_non_endpoint_edge_touch(
                ast,
                &node_renders,
                &layer_metrics,
                &gap_widths,
                &routes,
                box_height,
                &edge_gap_lanes,
            )?;

            if !has_touch {
                return Ok(Self {
                    options,
                    box_height,
                    layer_metrics,
                    gap_widths,
                    node_renders,
                    routes: routes.clone(),
                    edge_gap_lanes,
                    width,
                    height,
                });
            }

            fallback =
                Some((layer_metrics, gap_widths, node_renders, edge_gap_lanes, width, height));
            attempt_min_col_gap = attempt_min_col_gap.saturating_add(1);
        }

        let (layer_metrics, gap_widths, node_renders, edge_gap_lanes, width, height) =
            fallback.expect("at least one flowchart build attempt");

        Ok(Self {
            options,
            box_height,
            layer_metrics,
            gap_widths,
            node_renders,
            routes,
            edge_gap_lanes,
            width,
            height,
        })
    }

    fn render_text(&self, ast: &FlowchartAst) -> Result<String, FlowchartRenderError> {
        let mut canvas = Canvas::new(self.width, self.height)?;

        for (node_id, render) in &self.node_renders {
            let node = ast
                .nodes()
                .get(node_id)
                .ok_or_else(|| FlowchartRenderError::MissingNode { node_id: node_id.clone() })?;

            let layer = self
                .layer_metrics
                .get(render.layer)
                .ok_or(FlowchartRenderError::InvalidLayer { layer: render.layer })?;

            canvas.draw_box(render.box_x0, render.box_y0, render.box_x1, render.box_y1)?;

            let node_label = prefixed_object_label(node.label(), self.options);
            let clipped = truncate_with_ellipsis(&node_label, layer.inner_width);
            let clipped_len = text_len(&clipped);
            let left_pad = (layer.inner_width.saturating_sub(clipped_len)) / 2;
            let label_x = render.box_x0 + 1 + left_pad;
            canvas.write_str(label_x, render.box_y0 + 1, &clipped)?;

            if self.options.show_notes {
                if let Some(note) = node.note() {
                    let clipped = truncate_with_ellipsis(note, layer.inner_width);
                    let clipped_len = text_len(&clipped);
                    let left_pad = (layer.inner_width.saturating_sub(clipped_len)) / 2;
                    let note_x = render.box_x0 + 1 + left_pad;
                    canvas.write_str(note_x, render.box_y0 + 2, &clipped)?;
                }
            }
        }

        // Draw connectors after boxes. Vertical segments are drawn first, then horizontal
        // segments "bridge" across unrelated vertical segments by leaving gaps at crossings.
        for pass in [ConnectorDrawPass::Vertical, ConnectorDrawPass::Horizontal] {
            for (idx, (_edge_id, edge)) in ast.edges().iter().enumerate() {
                let from =
                    self.node_renders.get(edge.from_node_id()).copied().ok_or_else(|| {
                        FlowchartRenderError::MissingPlacement {
                            node_id: edge.from_node_id().clone(),
                        }
                    })?;
                let to = self.node_renders.get(edge.to_node_id()).copied().ok_or_else(|| {
                    FlowchartRenderError::MissingPlacement { node_id: edge.to_node_id().clone() }
                })?;

                if let Some(route) = self.routes.get(idx) {
                    draw_routed_connector(
                        &mut canvas,
                        from,
                        to,
                        &self.layer_metrics,
                        &self.gap_widths,
                        route,
                        self.box_height,
                        idx,
                        &self.edge_gap_lanes,
                        pass,
                    )?;
                } else {
                    draw_connector_pass(&mut canvas, from, to, pass)?;
                }
            }
        }

        Ok(canvas_to_string_trimmed(&canvas))
    }

    fn render_highlight_index(
        &self,
        diagram_id: &DiagramId,
        ast: &FlowchartAst,
    ) -> Result<HighlightIndex, FlowchartRenderError> {
        let flow_node_category =
            CategoryPath::new(vec!["flow".to_owned(), "node".to_owned()]).expect("valid");
        let flow_edge_category =
            CategoryPath::new(vec!["flow".to_owned(), "edge".to_owned()]).expect("valid");
        let flow_note_category =
            CategoryPath::new(vec!["flow".to_owned(), "note".to_owned()]).expect("valid");

        let mut highlight_index = HighlightIndex::new();

        for (node_id, render) in &self.node_renders {
            let node = ast
                .nodes()
                .get(node_id)
                .ok_or_else(|| FlowchartRenderError::MissingNode { node_id: node_id.clone() })?;
            let layer = self
                .layer_metrics
                .get(render.layer)
                .ok_or(FlowchartRenderError::InvalidLayer { layer: render.layer })?;

            let object_ref =
                ObjectRef::new(diagram_id.clone(), flow_node_category.clone(), node_id.clone());

            let mut spans = Vec::<LineSpan>::new();
            for y in render.box_y0..=render.box_y1 {
                spans.push((y, render.box_x0, render.box_x1));
            }
            spans.sort();
            spans.dedup();
            highlight_index.insert(object_ref, spans);

            if self.options.show_notes {
                if let Some(note) = node.note() {
                    let clipped = truncate_with_ellipsis(note, layer.inner_width);
                    let clipped_len = text_len(&clipped);
                    if clipped_len > 0 {
                        let left_pad = (layer.inner_width.saturating_sub(clipped_len)) / 2;
                        let note_x = render.box_x0 + 1 + left_pad;
                        let note_y = render.box_y0 + 2;
                        let note_ref = ObjectRef::new(
                            diagram_id.clone(),
                            flow_note_category.clone(),
                            node_id.clone(),
                        );
                        highlight_index.insert(
                            note_ref,
                            vec![(note_y, note_x, note_x + clipped_len.saturating_sub(1))],
                        );
                    }
                }
            }
        }

        let vertical_occupied = connector_vertical_occupancy_mask(
            ast,
            &self.layer_metrics,
            &self.gap_widths,
            &self.node_renders,
            &self.routes,
            self.box_height,
            &self.edge_gap_lanes,
            self.width,
            self.height,
        );

        for (idx, (edge_id, edge)) in ast.edges().iter().enumerate() {
            let from = self.node_renders.get(edge.from_node_id()).copied().ok_or_else(|| {
                FlowchartRenderError::MissingPlacement { node_id: edge.from_node_id().clone() }
            })?;
            let to = self.node_renders.get(edge.to_node_id()).copied().ok_or_else(|| {
                FlowchartRenderError::MissingPlacement { node_id: edge.to_node_id().clone() }
            })?;

            let mut spans = match self.routes.get(idx) {
                Some(route) => routed_connector_spans_bridged(
                    from,
                    to,
                    &self.layer_metrics,
                    &self.gap_widths,
                    route,
                    self.box_height,
                    idx,
                    &self.edge_gap_lanes,
                    &vertical_occupied,
                    self.width,
                ),
                None => connector_spans_bridged(from, to, &vertical_occupied, self.width),
            };

            spans.sort();
            spans.dedup();

            let object_ref =
                ObjectRef::new(diagram_id.clone(), flow_edge_category.clone(), edge_id.clone());
            highlight_index.insert(object_ref, spans);
        }

        Ok(highlight_index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowchartRenderError {
    Canvas(CanvasError),
    MissingNode { node_id: ObjectId },
    MissingPlacement { node_id: ObjectId },
    InvalidLayer { layer: usize },
}

impl fmt::Display for FlowchartRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canvas(err) => write!(f, "canvas error: {err}"),
            Self::MissingNode { node_id } => write!(f, "missing node {node_id} in AST"),
            Self::MissingPlacement { node_id } => write!(f, "missing placement for node {node_id}"),
            Self::InvalidLayer { layer } => write!(f, "invalid layer index: {layer}"),
        }
    }
}

impl std::error::Error for FlowchartRenderError {}

impl From<CanvasError> for FlowchartRenderError {
    fn from(value: CanvasError) -> Self {
        Self::Canvas(value)
    }
}

/// Deterministic baseline Unicode renderer for a flowchart.
///
/// This renderer consumes layered coordinates from `FlowchartLayout` and uses the AST only for node
/// labels.
///
/// Limitations (baseline):
/// - Edge routing uses `route_flowchart_edges_orthogonal` polylines, rendered through shared lanes
///   between layers.
/// - The router avoids node anchor points (not full box geometry), so dense graphs can still
///   produce overlapping connectors.
pub fn render_flowchart_unicode(
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
) -> Result<String, FlowchartRenderError> {
    render_flowchart_unicode_with_options(ast, layout, RenderOptions::default())
}

pub fn render_flowchart_unicode_with_options(
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
    options: RenderOptions,
) -> Result<String, FlowchartRenderError> {
    let plan = FlowchartRenderPlan::build(ast, layout, options)?;
    plan.render_text(ast)
}

pub fn render_flowchart_unicode_annotated(
    diagram_id: &DiagramId,
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
) -> Result<AnnotatedRender, FlowchartRenderError> {
    render_flowchart_unicode_annotated_with_options(
        diagram_id,
        ast,
        layout,
        RenderOptions::default(),
    )
}

pub fn render_flowchart_unicode_annotated_with_options(
    diagram_id: &DiagramId,
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
    options: RenderOptions,
) -> Result<AnnotatedRender, FlowchartRenderError> {
    let plan = FlowchartRenderPlan::build(ast, layout, options)?;
    let text = plan.render_text(ast)?;
    let mut highlight_index = plan.render_highlight_index(diagram_id, ast)?;

    clamp_highlight_index_to_text(&mut highlight_index, &text);
    Ok(AnnotatedRender { text, highlight_index })
}

// Extracted flowchart rendering internals and routing helpers.
include!("flowchart/helpers.rs");

#[cfg(test)]
mod tests;
