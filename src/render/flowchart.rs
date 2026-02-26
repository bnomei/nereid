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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EdgeCapCell {
    x: usize,
    y: usize,
    ch: char,
    outward_dx: i32,
    outward_dy: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct EdgeCapPlacement {
    start: Option<EdgeCapCell>,
    end: Option<EdgeCapCell>,
}

#[derive(Debug, Clone)]
struct FlowchartRenderPlan {
    options: RenderOptions,
    box_height: usize,
    layer_metrics: Vec<LayerMetrics>,
    gap_widths: Vec<usize>,
    node_renders: BTreeMap<ObjectId, NodeRender>,
    routes: Vec<Vec<GridPoint>>,
    edge_caps: Vec<EdgeCapPlacement>,
    edge_gap_lanes: Vec<Vec<Option<usize>>>,
    width: usize,
    height: usize,
}

fn overlay_edge_caps_on_text(mut text: String, edge_caps: &[EdgeCapPlacement]) -> String {
    const EDGE_LEFT: u8 = 1 << 0;
    const EDGE_RIGHT: u8 = 1 << 1;
    const EDGE_UP: u8 = 1 << 2;
    const EDGE_DOWN: u8 = 1 << 3;

    fn connector_mask(ch: char) -> Option<u8> {
        match ch {
            super::UNICODE_BOX_HORIZONTAL => Some(EDGE_LEFT | EDGE_RIGHT),
            super::UNICODE_BOX_VERTICAL => Some(EDGE_UP | EDGE_DOWN),
            super::UNICODE_BOX_TOP_LEFT => Some(EDGE_RIGHT | EDGE_DOWN),
            super::UNICODE_BOX_TOP_RIGHT => Some(EDGE_LEFT | EDGE_DOWN),
            super::UNICODE_BOX_BOTTOM_LEFT => Some(EDGE_RIGHT | EDGE_UP),
            super::UNICODE_BOX_BOTTOM_RIGHT => Some(EDGE_LEFT | EDGE_UP),
            super::UNICODE_BOX_TEE_RIGHT => Some(EDGE_UP | EDGE_DOWN | EDGE_RIGHT),
            super::UNICODE_BOX_TEE_LEFT => Some(EDGE_UP | EDGE_DOWN | EDGE_LEFT),
            super::UNICODE_BOX_TEE_DOWN => Some(EDGE_LEFT | EDGE_RIGHT | EDGE_DOWN),
            super::UNICODE_BOX_TEE_UP => Some(EDGE_LEFT | EDGE_RIGHT | EDGE_UP),
            super::UNICODE_BOX_CROSS => Some(EDGE_LEFT | EDGE_RIGHT | EDGE_UP | EDGE_DOWN),
            _ => None,
        }
    }

    fn connector_char(mask: u8) -> char {
        match mask {
            0 => ' ',
            1..=3 => super::UNICODE_BOX_HORIZONTAL,
            4 | 8 | 12 => super::UNICODE_BOX_VERTICAL,
            10 => super::UNICODE_BOX_TOP_LEFT,
            9 => super::UNICODE_BOX_TOP_RIGHT,
            6 => super::UNICODE_BOX_BOTTOM_LEFT,
            5 => super::UNICODE_BOX_BOTTOM_RIGHT,
            14 => super::UNICODE_BOX_TEE_RIGHT,
            13 => super::UNICODE_BOX_TEE_LEFT,
            11 => super::UNICODE_BOX_TEE_DOWN,
            7 => super::UNICODE_BOX_TEE_UP,
            15 => super::UNICODE_BOX_CROSS,
            _ => super::UNICODE_BOX_CROSS,
        }
    }

    fn step_cell(x: usize, y: usize, dx: i32, dy: i32) -> Option<(usize, usize)> {
        let nx = if dx < 0 {
            x.checked_sub(dx.unsigned_abs() as usize)?
        } else {
            x.checked_add(dx as usize)?
        };
        let ny = if dy < 0 {
            y.checked_sub(dy.unsigned_abs() as usize)?
        } else {
            y.checked_add(dy as usize)?
        };
        Some((nx, ny))
    }

    fn is_arrow_cap(ch: char) -> bool {
        matches!(ch, '◀' | '▶' | '▲' | '▼')
    }

    fn drop_connection_toward_cap(lines: &mut [Vec<char>], cap: EdgeCapCell) {
        if !is_arrow_cap(cap.ch) {
            return;
        }
        let toward_node_dx = -cap.outward_dx;
        let toward_node_dy = -cap.outward_dy;
        let Some((node_x, node_y)) = step_cell(cap.x, cap.y, toward_node_dx, toward_node_dy) else {
            return;
        };
        let Some(line) = lines.get_mut(node_y) else {
            return;
        };
        let Some(current) = line.get(node_x).copied() else {
            return;
        };
        let Some(mask) = connector_mask(current) else {
            return;
        };

        let bit_to_drop = if cap.outward_dx < 0 {
            EDGE_LEFT
        } else if cap.outward_dx > 0 {
            EDGE_RIGHT
        } else if cap.outward_dy < 0 {
            EDGE_UP
        } else if cap.outward_dy > 0 {
            EDGE_DOWN
        } else {
            0
        };
        if bit_to_drop == 0 || (mask & bit_to_drop) == 0 {
            return;
        }

        let new_mask = mask & !bit_to_drop;
        if new_mask == 0 {
            return;
        }
        line[node_x] = connector_char(new_mask);
    }

    let mut lines = text.lines().map(|line| line.chars().collect::<Vec<_>>()).collect::<Vec<_>>();

    for caps in edge_caps {
        if let Some(cap) = caps.start {
            drop_connection_toward_cap(&mut lines, cap);
            if let Some(line) = lines.get_mut(cap.y) {
                if cap.x < line.len() {
                    line[cap.x] = cap.ch;
                }
            }
        }
        if let Some(cap) = caps.end {
            drop_connection_toward_cap(&mut lines, cap);
            if let Some(line) = lines.get_mut(cap.y) {
                if cap.x < line.len() {
                    line[cap.x] = cap.ch;
                }
            }
        }
    }

    text.clear();
    for (idx, line) in lines.into_iter().enumerate() {
        if idx > 0 {
            text.push('\n');
        }
        for ch in line {
            text.push(ch);
        }
    }
    text
}

fn edge_has_vertical_segment_in_gap(
    route: &[GridPoint],
    gap_idx: usize,
    from_layer: usize,
    to_layer: usize,
    layer_count: usize,
) -> bool {
    if route.len() < 2 {
        return false;
    }

    for seg_idx in 0..route.len().saturating_sub(1) {
        let a = route[seg_idx];
        let b = route[seg_idx + 1];
        if a.x() != b.x() || a.y() == b.y() {
            continue;
        }

        let Some(seg_gap) =
            route_grid_x_to_lane_gap(route, seg_idx, from_layer, to_layer, layer_count)
        else {
            continue;
        };
        if seg_gap == gap_idx {
            return true;
        }
    }

    false
}

#[allow(clippy::too_many_arguments)]
fn normalize_edge_gap_lanes_for_bridge_alignment(
    ast: &FlowchartAst,
    node_renders: &BTreeMap<ObjectId, NodeRender>,
    routes: &[Vec<GridPoint>],
    edge_gap_lanes: &mut [Vec<Option<usize>>],
    gap_widths: &mut [usize],
    min_gap_width: usize,
    layer_count: usize,
) {
    let mut edge_layers = Vec::<Option<(usize, usize)>>::with_capacity(ast.edges().len());
    for (_edge_id, edge) in ast.edges() {
        let from_layer = node_renders.get(edge.from_node_id()).map(|render| render.layer);
        let to_layer = node_renders.get(edge.to_node_id()).map(|render| render.layer);
        edge_layers.push(from_layer.zip(to_layer));
    }

    for gap_idx in 0..gap_widths.len() {
        let mut assigned_edges = Vec::<(usize, usize)>::new();
        for (edge_idx, lanes) in edge_gap_lanes.iter().enumerate() {
            let Some(lane_idx) = lanes.get(gap_idx).copied().flatten() else {
                continue;
            };
            assigned_edges.push((edge_idx, lane_idx));
        }

        if assigned_edges.len() != 2 {
            continue;
        }

        let mut vertical_edges = Vec::<usize>::new();
        for (edge_idx, _lane_idx) in &assigned_edges {
            let Some((from_layer, to_layer)) = edge_layers.get(*edge_idx).copied().flatten() else {
                continue;
            };
            let Some(route) = routes.get(*edge_idx) else {
                continue;
            };
            if edge_has_vertical_segment_in_gap(route, gap_idx, from_layer, to_layer, layer_count) {
                vertical_edges.push(*edge_idx);
            }
        }

        if vertical_edges.len() != 1 {
            continue;
        }

        let vertical_edge_idx = vertical_edges[0];
        let horizontal_edge_idx = assigned_edges
            .iter()
            .find_map(|(edge_idx, _)| (*edge_idx != vertical_edge_idx).then_some(*edge_idx));
        let Some(horizontal_edge_idx) = horizontal_edge_idx else {
            continue;
        };

        let Some((from_layer, to_layer)) = edge_layers.get(vertical_edge_idx).copied().flatten()
        else {
            continue;
        };
        let forward = to_layer >= from_layer;

        let compact_gap_width = if min_gap_width >= 3 {
            // In TUI-style widened renders (extra gap enabled), keep one spare column at the
            // boundary-side of each forced lane in this compacted 2-edge scenario
            // (horizontal + vertical). This avoids visually hugging a node box while preserving
            // configured inter-lane clearance.
            min_gap_width.max(2 + LANE_MIN_X_CLEARANCE.max(1))
        } else {
            min_gap_width.max(1 + LANE_MIN_X_CLEARANCE.max(1))
        };
        let compact_candidates = gap_lane_x_candidates(0, compact_gap_width);
        if compact_candidates.len() < 2 {
            continue;
        }

        let mut sorted_candidates =
            compact_candidates.iter().copied().enumerate().collect::<Vec<_>>();
        sorted_candidates.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        let left_lane_idx = sorted_candidates.first().map(|(idx, _)| *idx).unwrap_or(0);
        let right_lane_idx = sorted_candidates.last().map(|(idx, _)| *idx).unwrap_or(0);
        let inset_left_lane_idx = sorted_candidates.get(1).map(|(idx, _)| *idx);
        let inset_right_lane_idx =
            sorted_candidates.get(sorted_candidates.len().saturating_sub(2)).map(|(idx, _)| *idx);

        let use_inset = min_gap_width >= 3 && compact_gap_width >= 4;
        let (vertical_lane_idx, horizontal_lane_idx) = if forward {
            (
                if use_inset {
                    inset_right_lane_idx.unwrap_or(right_lane_idx)
                } else {
                    right_lane_idx
                },
                left_lane_idx,
            )
        } else {
            (
                if use_inset {
                    inset_left_lane_idx.unwrap_or(left_lane_idx)
                } else {
                    left_lane_idx
                },
                right_lane_idx,
            )
        };

        for (edge_idx, lanes) in edge_gap_lanes.iter_mut().enumerate() {
            let Some(slot) = lanes.get_mut(gap_idx) else {
                continue;
            };
            let Some(_existing_lane) = *slot else {
                continue;
            };
            if edge_idx == vertical_edge_idx {
                *slot = Some(vertical_lane_idx);
            } else if edge_idx == horizontal_edge_idx {
                *slot = Some(horizontal_lane_idx);
            }
        }

        gap_widths[gap_idx] = compact_gap_width;
    }
}

fn align_routes_to_endpoint_rows(
    ast: &FlowchartAst,
    node_renders: &BTreeMap<ObjectId, NodeRender>,
    routes: &[Vec<GridPoint>],
) -> Vec<Vec<GridPoint>> {
    let mut aligned = routes.to_vec();

    for (edge_idx, (_edge_id, edge)) in ast.edges().iter().enumerate() {
        let Some(route) = aligned.get_mut(edge_idx) else {
            continue;
        };
        if route.is_empty() {
            continue;
        }

        let Some(from) = node_renders.get(edge.from_node_id()).copied() else {
            continue;
        };
        let Some(to) = node_renders.get(edge.to_node_id()).copied() else {
            continue;
        };

        let from_grid_y = (from.index_in_layer as i32).saturating_mul(2);
        let to_grid_y = (to.index_in_layer as i32).saturating_mul(2);
        let min_grid_y = from_grid_y.min(to_grid_y);
        let max_grid_y = if from_grid_y == to_grid_y {
            from_grid_y.saturating_add(1)
        } else {
            from_grid_y.max(to_grid_y)
        };

        for point in route.iter_mut() {
            let clamped_y = point.y().clamp(min_grid_y, max_grid_y);
            *point = GridPoint::new(point.x(), clamped_y);
        }

        route[0] = GridPoint::new(route[0].x(), from_grid_y);
        let last_idx = route.len().saturating_sub(1);
        route[last_idx] = GridPoint::new(route[last_idx].x(), to_grid_y);

        route.dedup();
    }

    aligned
}

impl FlowchartRenderPlan {
    fn build(
        ast: &FlowchartAst,
        layout: &FlowchartLayout,
        options: RenderOptions,
    ) -> Result<Self, FlowchartRenderError> {
        let box_height = flow_box_height(options);
        let raw_routes = route_flowchart_edges_orthogonal_key_order(ast, layout);
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

            let (mut edge_gap_lanes, mut gap_widths) = assign_edge_gap_lanes(
                ast,
                &initial_node_renders,
                initial_layer_metrics.len(),
                &raw_routes,
                box_height,
                attempt_min_col_gap,
            );
            normalize_edge_gap_lanes_for_bridge_alignment(
                ast,
                &initial_node_renders,
                &raw_routes,
                &mut edge_gap_lanes,
                &mut gap_widths,
                attempt_min_col_gap,
                initial_layer_metrics.len(),
            );

            let layer_metrics = layer_metrics(ast, layout, &gap_widths, options)?;
            let (node_renders, _base_height) = node_renders(layout, &layer_metrics, box_height)?;
            let routes = align_routes_to_endpoint_rows(ast, &node_renders, &raw_routes);

            let width = layer_metrics.last().map(|layer| layer.x1 + 1).unwrap_or(1);
            let height = routed_height(base_height, &raw_routes, box_height);

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
                let edge_caps = assign_edge_cap_placements(
                    ast,
                    &node_renders,
                    &layer_metrics,
                    &gap_widths,
                    &routes,
                    box_height,
                    &edge_gap_lanes,
                    options.flowchart_extra_col_gap > 0,
                );
                return Ok(Self {
                    options,
                    box_height,
                    layer_metrics,
                    gap_widths,
                    node_renders,
                    routes,
                    edge_caps,
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
        let routes = align_routes_to_endpoint_rows(ast, &node_renders, &raw_routes);
        let edge_caps = assign_edge_cap_placements(
            ast,
            &node_renders,
            &layer_metrics,
            &gap_widths,
            &routes,
            box_height,
            &edge_gap_lanes,
            options.flowchart_extra_col_gap > 0,
        );

        Ok(Self {
            options,
            box_height,
            layer_metrics,
            gap_widths,
            node_renders,
            routes,
            edge_caps,
            edge_gap_lanes,
            width,
            height,
        })
    }

    fn render_text(&self, ast: &FlowchartAst) -> Result<String, FlowchartRenderError> {
        let mut canvas = Canvas::new(self.width, self.height)?;

        for render in self.node_renders.values() {
            canvas.draw_box(render.box_x0, render.box_y0, render.box_x1, render.box_y1)?;
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

        if self.options.flowchart_extra_col_gap > 0 {
            for caps in &self.edge_caps {
                refine_edge_cap_tails(&mut canvas, caps)?;
            }
        }

        // Draw labels last so routed connectors can never clobber node text cells.
        for (node_id, render) in &self.node_renders {
            let node = ast
                .nodes()
                .get(node_id)
                .ok_or_else(|| FlowchartRenderError::MissingNode { node_id: node_id.clone() })?;

            let layer = self
                .layer_metrics
                .get(render.layer)
                .ok_or(FlowchartRenderError::InvalidLayer { layer: render.layer })?;

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

        let text = canvas_to_string_trimmed(&canvas);
        Ok(overlay_edge_caps_on_text(text, &self.edge_caps))
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

            if let Some(caps) = self.edge_caps.get(idx) {
                if let Some(cap) = caps.start {
                    spans.push((cap.y, cap.x, cap.x));
                }
                if let Some(cap) = caps.end {
                    spans.push((cap.y, cap.x, cap.x));
                }
            }

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
