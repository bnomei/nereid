// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

/// Flowchart rendering internals:
/// routing spans, lane assignment, collision checks, and connector drawing passes.
#[cfg(test)]
fn hline_span(y: usize, x0: usize, x1: usize) -> LineSpan {
    if x0 <= x1 {
        (y, x0, x1)
    } else {
        (y, x1, x0)
    }
}

fn vline_spans(x: usize, y0: usize, y1: usize) -> Vec<LineSpan> {
    let (min_y, max_y) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    (min_y..=max_y).map(|y| (y, x, x)).collect()
}

#[cfg(test)]
fn connector_spans(from: NodeRender, to: NodeRender) -> Vec<LineSpan> {
    let from_x = from.box_x1;
    let from_y = from.mid_y();
    let to_x = to.box_x0;
    let to_y = to.mid_y();

    if from_y == to_y {
        return vec![hline_span(from_y, from_x, to_x)];
    }

    let bend_x = (from_x + to_x) / 2;
    let mut spans = Vec::<LineSpan>::new();
    spans.push(hline_span(from_y, from_x, bend_x));
    spans.extend(vline_spans(bend_x, from_y, to_y));
    spans.push(hline_span(to_y, bend_x, to_x));
    spans
}

#[cfg(test)]
fn routed_connector_spans(
    from: NodeRender,
    to: NodeRender,
    layer_metrics: &[LayerMetrics],
    gap_widths: &[usize],
    route: &[GridPoint],
    box_height: usize,
    edge_idx: usize,
    edge_gap_lanes: &[Vec<Option<usize>>],
) -> Vec<LineSpan> {
    if route.len() < 2 {
        return connector_spans(from, to);
    }

    let mut points = Vec::<(usize, usize)>::with_capacity(route.len());
    for (idx, p) in route.iter().enumerate() {
        let x = match route_grid_x_to_lane_x(
            route,
            idx,
            from,
            to,
            layer_metrics,
            gap_widths,
            edge_idx,
            edge_gap_lanes,
        ) {
            Some(x) => x,
            None => return connector_spans(from, to),
        };
        let y = grid_y_to_canvas_y(p.y(), box_height);
        let point = (x, y);
        if points.last() != Some(&point) {
            points.push(point);
        }
    }

    if points.is_empty() {
        return connector_spans(from, to);
    }

    let mut spans = Vec::<LineSpan>::new();

    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if x0 == x1 && y0 == y1 {
            continue;
        }
        if x0 == x1 {
            spans.extend(vline_spans(x0, y0, y1));
        } else if y0 == y1 {
            spans.push(hline_span(y0, x0, x1));
        } else {
            spans.push(hline_span(y0, x0, x1));
            spans.extend(vline_spans(x1, y0, y1));
        }
    }

    // Stubs connect the routed polyline (in lane space) to the node boxes.
    let (start_x, start_y) = points[0];
    let from_y = from.mid_y();
    let from_x = if start_x >= from.box_x1 {
        from.box_x1
    } else {
        from.box_x0
    };
    spans.push(hline_span(from_y, from_x, start_x));

    let (end_x, end_y) = *points.last().expect("non-empty");
    let to_y = to.mid_y();
    let to_x = if end_x <= to.box_x0 {
        to.box_x0
    } else {
        to.box_x1
    };
    spans.push(hline_span(to_y, end_x, to_x));

    // Ensure the stub y matches the lane y (defensive: should already match for endpoints).
    if start_y != from_y {
        spans.extend(vline_spans(start_x, start_y, from_y));
    }
    if end_y != to_y {
        spans.extend(vline_spans(end_x, end_y, to_y));
    }

    spans
}

fn projected_route_points(
    route: &[GridPoint],
    from: NodeRender,
    to: NodeRender,
    layer_metrics: &[LayerMetrics],
    gap_widths: &[usize],
    box_height: usize,
    edge_idx: usize,
    edge_gap_lanes: &[Vec<Option<usize>>],
) -> Option<Vec<(usize, usize)>> {
    if route.len() < 2 {
        return None;
    }

    let mut points = Vec::<(usize, usize)>::with_capacity(route.len());
    for (idx, p) in route.iter().enumerate() {
        let x = route_grid_x_to_lane_x(
            route,
            idx,
            from,
            to,
            layer_metrics,
            gap_widths,
            edge_idx,
            edge_gap_lanes,
        )?;
        let y = grid_y_to_canvas_y(p.y(), box_height);
        let point = (x, y);
        if points.last() != Some(&point) {
            points.push(point);
        }
    }

    (!points.is_empty()).then_some(points)
}

fn connector_vertical_occupancy_mask(
    ast: &FlowchartAst,
    layer_metrics: &[LayerMetrics],
    gap_widths: &[usize],
    node_renders: &BTreeMap<ObjectId, NodeRender>,
    routes: &[Vec<GridPoint>],
    box_height: usize,
    edge_gap_lanes: &[Vec<Option<usize>>],
    width: usize,
    height: usize,
) -> Vec<bool> {
    let mut occupied = vec![false; width.saturating_mul(height)];

    fn mark_cell(occupied: &mut [bool], width: usize, height: usize, x: usize, y: usize) {
        if x < width && y < height {
            occupied[(y * width) + x] = true;
        }
    }

    fn mark_vline(
        occupied: &mut [bool],
        width: usize,
        height: usize,
        x: usize,
        y0: usize,
        y1: usize,
    ) {
        if x >= width {
            return;
        }
        let (min_y, max_y) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for y in min_y..=max_y {
            if y >= height {
                continue;
            }
            occupied[(y * width) + x] = true;
        }
    }

    // Node boxes contribute vertical edges along their left/right borders.
    for render in node_renders.values() {
        for y in render.box_y0..=render.box_y1 {
            mark_cell(&mut occupied, width, height, render.box_x0, y);
            mark_cell(&mut occupied, width, height, render.box_x1, y);
        }
    }

    for (edge_idx, (_edge_id, edge)) in ast.edges().iter().enumerate() {
        let Some(from) = node_renders.get(edge.from_node_id()).copied() else {
            continue;
        };
        let Some(to) = node_renders.get(edge.to_node_id()).copied() else {
            continue;
        };

        let points = routes.get(edge_idx).and_then(|route| {
            projected_route_points(
                route,
                from,
                to,
                layer_metrics,
                gap_widths,
                box_height,
                edge_idx,
                edge_gap_lanes,
            )
        });

        if let Some(points) = points {
            for pair in points.windows(2) {
                let (x0, y0) = pair[0];
                let (x1, y1) = pair[1];
                if x0 == x1 {
                    mark_vline(&mut occupied, width, height, x0, y0, y1);
                } else if y0 != y1 {
                    // Shouldn't happen: routed polylines are orthogonal. Mirror the renderer's
                    // deterministic fallback (an L).
                    mark_vline(&mut occupied, width, height, x1, y0, y1);
                }
            }

            let (start_x, start_y) = points.first().copied().expect("non-empty");
            let from_y = from.mid_y();
            if start_y != from_y {
                mark_vline(&mut occupied, width, height, start_x, start_y, from_y);
            }

            let (end_x, end_y) = points.last().copied().expect("non-empty");
            let to_y = to.mid_y();
            if end_y != to_y {
                mark_vline(&mut occupied, width, height, end_x, end_y, to_y);
            }
            continue;
        }

        // Fallback connector (same logic as `draw_connector_pass` vertical).
        let from_y = from.mid_y();
        let to_y = to.mid_y();
        if from_y != to_y {
            let from_x = from.box_x1;
            let to_x = to.box_x0;
            let bend_x = (from_x + to_x) / 2;
            mark_vline(&mut occupied, width, height, bend_x, from_y, to_y);
        }
    }

    occupied
}

fn hline_spans_bridged(
    y: usize,
    x0: usize,
    x1: usize,
    vertical_occupied: &[bool],
    width: usize,
) -> Vec<LineSpan> {
    if width == 0 {
        return Vec::new();
    }

    let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    let mut out = Vec::<LineSpan>::new();
    let mut run_start: Option<usize> = None;

    for x in min_x..=max_x {
        let is_endpoint = x == min_x || x == max_x;
        let idx = y.saturating_mul(width).saturating_add(x);
        let should_draw = is_endpoint || !vertical_occupied.get(idx).copied().unwrap_or(false);

        if should_draw {
            if run_start.is_none() {
                run_start = Some(x);
            }
        } else if let Some(start) = run_start.take() {
            out.push((y, start, x.saturating_sub(1)));
        }
    }

    if let Some(start) = run_start {
        out.push((y, start, max_x));
    }

    out
}

fn connector_spans_bridged(
    from: NodeRender,
    to: NodeRender,
    vertical_occupied: &[bool],
    width: usize,
) -> Vec<LineSpan> {
    let from_x = from.box_x1;
    let from_y = from.mid_y();
    let to_x = to.box_x0;
    let to_y = to.mid_y();

    if from_y == to_y {
        return hline_spans_bridged(from_y, from_x, to_x, vertical_occupied, width);
    }

    let bend_x = (from_x + to_x) / 2;
    let mut spans = Vec::<LineSpan>::new();
    spans.extend(hline_spans_bridged(
        from_y,
        from_x,
        bend_x,
        vertical_occupied,
        width,
    ));
    spans.extend(vline_spans(bend_x, from_y, to_y));
    spans.extend(hline_spans_bridged(
        to_y,
        bend_x,
        to_x,
        vertical_occupied,
        width,
    ));
    spans
}

fn routed_connector_spans_bridged(
    from: NodeRender,
    to: NodeRender,
    layer_metrics: &[LayerMetrics],
    gap_widths: &[usize],
    route: &[GridPoint],
    box_height: usize,
    edge_idx: usize,
    edge_gap_lanes: &[Vec<Option<usize>>],
    vertical_occupied: &[bool],
    width: usize,
) -> Vec<LineSpan> {
    let Some(points) = projected_route_points(
        route,
        from,
        to,
        layer_metrics,
        gap_widths,
        box_height,
        edge_idx,
        edge_gap_lanes,
    ) else {
        return connector_spans_bridged(from, to, vertical_occupied, width);
    };

    let mut spans = Vec::<LineSpan>::new();

    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if x0 == x1 && y0 == y1 {
            continue;
        }
        if x0 == x1 {
            spans.extend(vline_spans(x0, y0, y1));
        } else if y0 == y1 {
            spans.extend(hline_spans_bridged(y0, x0, x1, vertical_occupied, width));
        } else {
            // Routing should be orthogonal; fall back to a deterministic L to avoid crashing.
            spans.extend(hline_spans_bridged(y0, x0, x1, vertical_occupied, width));
            spans.extend(vline_spans(x1, y0, y1));
        }
    }

    // Stubs connect the routed polyline (in lane space) to the node boxes.
    let (start_x, start_y) = points[0];
    let from_y = from.mid_y();
    let from_x = if start_x >= from.box_x1 {
        from.box_x1
    } else {
        from.box_x0
    };
    spans.extend(hline_spans_bridged(
        from_y,
        from_x,
        start_x,
        vertical_occupied,
        width,
    ));

    let (end_x, end_y) = *points.last().expect("non-empty");
    let to_y = to.mid_y();
    let to_x = if end_x <= to.box_x0 {
        to.box_x0
    } else {
        to.box_x1
    };
    spans.extend(hline_spans_bridged(
        to_y,
        end_x,
        to_x,
        vertical_occupied,
        width,
    ));

    // Ensure the stub y matches the lane y (defensive: should already match for endpoints).
    if start_y != from_y {
        spans.extend(vline_spans(start_x, start_y, from_y));
    }
    if end_y != to_y {
        spans.extend(vline_spans(end_x, end_y, to_y));
    }

    spans
}

fn routed_height(base_height: usize, routes: &[Vec<GridPoint>], box_height: usize) -> usize {
    let mut height = base_height;
    for route in routes {
        for p in route {
            let y = grid_y_to_canvas_y(p.y(), box_height);
            height = height.max(y + 1);
        }
    }
    height
}

fn layer_metrics(
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
    gap_widths: &[usize],
    options: RenderOptions,
) -> Result<Vec<LayerMetrics>, FlowchartRenderError> {
    let mut out = Vec::<LayerMetrics>::with_capacity(layout.layers().len());
    let mut cursor_x = 0usize;

    for (layer_idx, layer_nodes) in layout.layers().iter().enumerate() {
        let max_label_len = layer_nodes
            .iter()
            .map(|node_id| {
                ast.nodes()
                    .get(node_id)
                    .ok_or_else(|| FlowchartRenderError::MissingNode {
                        node_id: node_id.clone(),
                    })
                    .map(|node| text_len(&prefixed_object_label(node.label(), options)))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap_or(0);

        let mut inner_width = (max_label_len + 2).max(MIN_BOX_INNER_WIDTH);
        let mut total_width = inner_width + 2;

        // Keep widths odd so connectors naturally align to center cells.
        if total_width % 2 == 0 {
            total_width += 1;
            inner_width += 1;
        }

        let x0 = cursor_x;
        let x1 = x0 + total_width - 1;
        out.push(LayerMetrics {
            x0,
            x1,
            inner_width,
            total_width,
        });
        let gap_width = gap_widths.get(layer_idx).copied().unwrap_or(0);
        cursor_x = x1 + 1 + gap_width;
    }

    Ok(out)
}

fn prefixed_object_label(label: &str, options: RenderOptions) -> String {
    if options.prefix_object_labels {
        format!("{OBJECT_LABEL_PREFIX}{label}")
    } else {
        label.to_owned()
    }
}

fn node_renders(
    layout: &FlowchartLayout,
    layer_metrics: &[LayerMetrics],
    box_height: usize,
) -> Result<(BTreeMap<ObjectId, NodeRender>, usize), FlowchartRenderError> {
    let max_nodes_in_any_layer = layout
        .layers()
        .iter()
        .map(|layer| layer.len())
        .max()
        .unwrap_or(0);
    let height = if max_nodes_in_any_layer == 0 {
        1
    } else {
        (max_nodes_in_any_layer * box_height) + ((max_nodes_in_any_layer - 1) * ROW_GAP)
    };

    let mut renders = BTreeMap::<ObjectId, NodeRender>::new();
    for (layer_idx, layer_nodes) in layout.layers().iter().enumerate() {
        let layer = layer_metrics
            .get(layer_idx)
            .ok_or(FlowchartRenderError::InvalidLayer { layer: layer_idx })?;

        for (index_in_layer, node_id) in layer_nodes.iter().enumerate() {
            let y0 = index_in_layer * (box_height + ROW_GAP);
            let y1 = y0 + box_height - 1;
            renders.insert(
                node_id.clone(),
                NodeRender {
                    layer: layer_idx,
                    index_in_layer,
                    box_x0: layer.x0,
                    box_x1: layer.x1,
                    box_y0: y0,
                    box_y1: y1,
                },
            );
        }
    }

    Ok((renders, height))
}

#[derive(Debug, Clone)]
struct EdgeGapUsage {
    edge_idx: usize,
    min_y: usize,
    max_y: usize,
    intervals: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, Default)]
struct EdgeGapEndpoints {
    start_gap: Option<usize>,
    end_gap: Option<usize>,
    from_y: usize,
    to_y: usize,
}

fn assign_edge_gap_lanes(
    ast: &FlowchartAst,
    node_renders: &BTreeMap<ObjectId, NodeRender>,
    layer_count: usize,
    routes: &[Vec<GridPoint>],
    box_height: usize,
    min_gap_width: usize,
) -> (Vec<Vec<Option<usize>>>, Vec<usize>) {
    assign_edge_gap_lanes_with_clearance(
        ast,
        node_renders,
        layer_count,
        routes,
        box_height,
        min_gap_width,
    )
}

#[allow(dead_code)]
fn assign_edge_gap_lanes_classic(
    ast: &FlowchartAst,
    node_renders: &BTreeMap<ObjectId, NodeRender>,
    layer_count: usize,
    routes: &[Vec<GridPoint>],
    box_height: usize,
    min_gap_width: usize,
) -> (Vec<Vec<Option<usize>>>, Vec<usize>) {
    let edge_count = ast.edges().len();
    let gap_count = layer_count.saturating_sub(1);
    let mut edge_gap_lanes = vec![vec![None; gap_count]; edge_count];
    let mut gap_widths = vec![min_gap_width; gap_count];
    let mut endpoints_by_edge = vec![EdgeGapEndpoints::default(); edge_count];
    let mut vertical_intervals_by_edge =
        vec![vec![Vec::<(usize, usize)>::new(); gap_count]; edge_count];

    if edge_count == 0 || gap_count == 0 {
        return (edge_gap_lanes, gap_widths);
    }

    let mut usages_by_gap = vec![Vec::<EdgeGapUsage>::new(); gap_count];

    for (edge_idx, (_edge_id, edge)) in ast.edges().iter().enumerate() {
        let Some(from) = node_renders.get(edge.from_node_id()).copied() else {
            continue;
        };
        let Some(to) = node_renders.get(edge.to_node_id()).copied() else {
            continue;
        };
        let Some(route) = routes.get(edge_idx).map(|route| route.as_slice()) else {
            continue;
        };

        if !route.is_empty() {
            let last_idx = route.len().saturating_sub(1);
            endpoints_by_edge[edge_idx] = EdgeGapEndpoints {
                start_gap: route_grid_x_to_lane_gap(route, 0, from.layer, to.layer, layer_count),
                end_gap: route_grid_x_to_lane_gap(
                    route,
                    last_idx,
                    from.layer,
                    to.layer,
                    layer_count,
                ),
                from_y: from.mid_y(),
                to_y: to.mid_y(),
            };
        }

        let mut intervals_per_gap = vec![Vec::<(usize, usize)>::new(); gap_count];

        for seg_idx in 0..route.len().saturating_sub(1) {
            let a = route[seg_idx];
            let b = route[seg_idx + 1];
            if a.x() != b.x() || a.y() == b.y() {
                continue;
            }

            let Some(gap_idx) =
                route_grid_x_to_lane_gap(route, seg_idx, from.layer, to.layer, layer_count)
            else {
                continue;
            };
            if gap_idx >= gap_count {
                continue;
            }

            let y0 = grid_y_to_canvas_y(a.y(), box_height);
            let y1 = grid_y_to_canvas_y(b.y(), box_height);
            intervals_per_gap[gap_idx].push((y0.min(y1), y0.max(y1)));
        }

        for gap_idx in 0..gap_count {
            if intervals_per_gap[gap_idx].is_empty() {
                continue;
            }
            let intervals = merge_intervals(std::mem::take(&mut intervals_per_gap[gap_idx]));
            vertical_intervals_by_edge[edge_idx][gap_idx] = intervals.clone();
            let mut min_y = usize::MAX;
            let mut max_y = 0usize;
            for (y0, y1) in &intervals {
                min_y = min_y.min(*y0);
                max_y = max_y.max(*y1);
            }
            usages_by_gap[gap_idx].push(EdgeGapUsage {
                edge_idx,
                min_y,
                max_y,
                intervals,
            });
        }
    }

    for gap_idx in 0..gap_count {
        let usages = &mut usages_by_gap[gap_idx];
        if usages.is_empty() {
            continue;
        }

        usages.sort_by(|a, b| {
            a.min_y
                .cmp(&b.min_y)
                .then_with(|| a.max_y.cmp(&b.max_y))
                .then_with(|| a.edge_idx.cmp(&b.edge_idx))
        });

        let mut lane_occupied = Vec::<Vec<(usize, usize)>>::new();

        for usage in usages.iter() {
            let mut assigned = None;
            for lane_idx in 0..lane_occupied.len() {
                if !intervals_overlap(&usage.intervals, &lane_occupied[lane_idx]) {
                    assigned = Some(lane_idx);
                    break;
                }
            }

            let lane_idx = match assigned {
                Some(lane_idx) => lane_idx,
                None => {
                    lane_occupied.push(Vec::new());
                    lane_occupied.len().saturating_sub(1)
                }
            };
            edge_gap_lanes[usage.edge_idx][gap_idx] = Some(lane_idx);

            let mut merged = lane_occupied[lane_idx].clone();
            merged.extend_from_slice(&usage.intervals);
            lane_occupied[lane_idx] = merge_intervals(merged);
        }

        gap_widths[gap_idx] = gap_widths[gap_idx].max(lane_occupied.len());

        let gap_width = gap_widths[gap_idx];
        let candidates = gap_lane_x_candidates(0, gap_width);

        let edge_x = |edge_idx: usize, candidates: &[usize]| -> usize {
            edge_gap_lanes
                .get(edge_idx)
                .and_then(|lanes| lanes.get(gap_idx))
                .copied()
                .flatten()
                .and_then(|lane_idx| candidates.get(lane_idx).copied())
                .unwrap_or_else(|| candidates[0])
        };

        let intervals_cover_y = |intervals: &[(usize, usize)], y: usize| -> bool {
            intervals.iter().any(|(y0, y1)| *y0 <= y && y <= *y1)
        };

        // Check whether the current lane assignment would force any "bridge" crossings at stub
        // rows. When needed, we re-run lane assignment with stub rows treated as occupied and
        // reorder lanes to keep upper routes to the right.
        let mut needs_enhancement = false;
        for (edge_idx, endpoints) in endpoints_by_edge.iter().enumerate() {
            let start_here = endpoints.start_gap == Some(gap_idx);
            let end_here = endpoints.end_gap == Some(gap_idx);
            if !start_here && !end_here {
                continue;
            }

            let x_b = edge_x(edge_idx, &candidates);

            if start_here {
                let y = endpoints.from_y;
                for usage in usages.iter() {
                    if usage.edge_idx == edge_idx {
                        continue;
                    }
                    if !intervals_cover_y(&usage.intervals, y) {
                        continue;
                    }
                    let x_a = edge_x(usage.edge_idx, &candidates);
                    if x_a < x_b {
                        needs_enhancement = true;
                        break;
                    }
                }
            }

            if !needs_enhancement && end_here {
                let y = endpoints.to_y;
                for usage in usages.iter() {
                    if usage.edge_idx == edge_idx {
                        continue;
                    }
                    if !intervals_cover_y(&usage.intervals, y) {
                        continue;
                    }
                    let x_a = edge_x(usage.edge_idx, &candidates);
                    if x_a > x_b {
                        needs_enhancement = true;
                        break;
                    }
                }
            }

            if needs_enhancement {
                break;
            }
        }

        if !needs_enhancement {
            continue;
        }

        // Re-run lane assignment for this gap including stub rows as occupied.
        for edge_idx in 0..edge_count {
            edge_gap_lanes[edge_idx][gap_idx] = None;
        }

        let mut enhanced_usages = Vec::<EdgeGapUsage>::new();
        for edge_idx in 0..edge_count {
            let endpoints = endpoints_by_edge[edge_idx];
            let mut intervals = vertical_intervals_by_edge[edge_idx][gap_idx].clone();

            if endpoints.start_gap == Some(gap_idx) {
                intervals.push((endpoints.from_y, endpoints.from_y));
            }
            if endpoints.end_gap == Some(gap_idx) {
                intervals.push((endpoints.to_y, endpoints.to_y));
            }

            if intervals.is_empty() {
                continue;
            }

            let intervals = merge_intervals(intervals);
            let mut min_y = usize::MAX;
            let mut max_y = 0usize;
            for (y0, y1) in &intervals {
                min_y = min_y.min(*y0);
                max_y = max_y.max(*y1);
            }

            enhanced_usages.push(EdgeGapUsage {
                edge_idx,
                min_y,
                max_y,
                intervals,
            });
        }

        enhanced_usages.sort_by(|a, b| {
            a.min_y
                .cmp(&b.min_y)
                .then_with(|| a.max_y.cmp(&b.max_y))
                .then_with(|| a.edge_idx.cmp(&b.edge_idx))
        });

        let mut lane_occupied = Vec::<Vec<(usize, usize)>>::new();
        for usage in enhanced_usages.iter() {
            let mut assigned = None;
            for lane_idx in 0..lane_occupied.len() {
                if !intervals_overlap(&usage.intervals, &lane_occupied[lane_idx]) {
                    assigned = Some(lane_idx);
                    break;
                }
            }

            let lane_idx = match assigned {
                Some(lane_idx) => lane_idx,
                None => {
                    lane_occupied.push(Vec::new());
                    lane_occupied.len().saturating_sub(1)
                }
            };
            edge_gap_lanes[usage.edge_idx][gap_idx] = Some(lane_idx);

            let mut merged = lane_occupied[lane_idx].clone();
            merged.extend_from_slice(&usage.intervals);
            lane_occupied[lane_idx] = merge_intervals(merged);
        }

        gap_widths[gap_idx] = gap_widths[gap_idx].max(lane_occupied.len());

        if lane_occupied.len() > 1 {
            let lane_count = lane_occupied.len();

            let mut lanes_by_y = lane_occupied
                .iter()
                .enumerate()
                .map(|(lane_idx, intervals)| {
                    let min_y = intervals
                        .iter()
                        .map(|(y0, _y1)| *y0)
                        .min()
                        .unwrap_or(usize::MAX);
                    (min_y, lane_idx)
                })
                .collect::<Vec<_>>();

            // Place lanes with lower min_y (visually higher) further to the right so stubs from
            // lower nodes are less likely to bridge across them.
            lanes_by_y.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

            let gap_width = gap_widths[gap_idx].max(lane_count);
            let candidates = gap_lane_x_candidates(0, gap_width);
            let mut candidates_by_x = (0..lane_count)
                .map(|candidate_idx| (candidates[candidate_idx], candidate_idx))
                .collect::<Vec<_>>();
            candidates_by_x.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

            let mut candidate_by_lane = vec![0usize; lane_count];
            for (rank, (_min_y, lane_idx)) in lanes_by_y.into_iter().enumerate() {
                let candidate_idx = candidates_by_x[rank].1;
                candidate_by_lane[lane_idx] = candidate_idx;
            }

            for usage in enhanced_usages.iter() {
                if let Some(lane_idx) = edge_gap_lanes[usage.edge_idx][gap_idx] {
                    edge_gap_lanes[usage.edge_idx][gap_idx] = Some(candidate_by_lane[lane_idx]);
                }
            }
        }
    }

    (edge_gap_lanes, gap_widths)
}

fn assign_edge_gap_lanes_with_clearance(
    ast: &FlowchartAst,
    node_renders: &BTreeMap<ObjectId, NodeRender>,
    layer_count: usize,
    routes: &[Vec<GridPoint>],
    box_height: usize,
    min_gap_width: usize,
) -> (Vec<Vec<Option<usize>>>, Vec<usize>) {
    let edge_count = ast.edges().len();
    let gap_count = layer_count.saturating_sub(1);
    let mut edge_gap_lanes = vec![vec![None; gap_count]; edge_count];
    let mut gap_widths = vec![min_gap_width; gap_count];
    let mut endpoints_by_edge = vec![EdgeGapEndpoints::default(); edge_count];
    let mut vertical_intervals_by_edge =
        vec![vec![Vec::<(usize, usize)>::new(); gap_count]; edge_count];

    if edge_count == 0 || gap_count == 0 {
        return (edge_gap_lanes, gap_widths);
    }

    for (edge_idx, (_edge_id, edge)) in ast.edges().iter().enumerate() {
        let Some(from) = node_renders.get(edge.from_node_id()).copied() else {
            continue;
        };
        let Some(to) = node_renders.get(edge.to_node_id()).copied() else {
            continue;
        };
        let Some(route) = routes.get(edge_idx).map(|route| route.as_slice()) else {
            continue;
        };

        if !route.is_empty() {
            let last_idx = route.len().saturating_sub(1);
            endpoints_by_edge[edge_idx] = EdgeGapEndpoints {
                start_gap: route_grid_x_to_lane_gap(route, 0, from.layer, to.layer, layer_count),
                end_gap: route_grid_x_to_lane_gap(
                    route,
                    last_idx,
                    from.layer,
                    to.layer,
                    layer_count,
                ),
                from_y: from.mid_y(),
                to_y: to.mid_y(),
            };
        }

        let mut intervals_per_gap = vec![Vec::<(usize, usize)>::new(); gap_count];

        for seg_idx in 0..route.len().saturating_sub(1) {
            let a = route[seg_idx];
            let b = route[seg_idx + 1];
            if a.x() != b.x() || a.y() == b.y() {
                continue;
            }

            let Some(gap_idx) =
                route_grid_x_to_lane_gap(route, seg_idx, from.layer, to.layer, layer_count)
            else {
                continue;
            };
            if gap_idx >= gap_count {
                continue;
            }

            let y0 = grid_y_to_canvas_y(a.y(), box_height);
            let y1 = grid_y_to_canvas_y(b.y(), box_height);
            intervals_per_gap[gap_idx].push((y0.min(y1), y0.max(y1)));
        }

        for gap_idx in 0..gap_count {
            if intervals_per_gap[gap_idx].is_empty() {
                continue;
            }
            vertical_intervals_by_edge[edge_idx][gap_idx] =
                merge_intervals(std::mem::take(&mut intervals_per_gap[gap_idx]));
        }
    }

    for gap_idx in 0..gap_count {
        let mut usages = Vec::<EdgeGapUsage>::new();
        for edge_idx in 0..edge_count {
            let mut intervals = vertical_intervals_by_edge[edge_idx][gap_idx].clone();
            let endpoints = endpoints_by_edge[edge_idx];

            if endpoints.start_gap == Some(gap_idx) {
                intervals.push(expand_row_interval(
                    endpoints.from_y,
                    STUB_ROW_KEEPOUT_RADIUS,
                ));
            }
            if endpoints.end_gap == Some(gap_idx) {
                intervals.push(expand_row_interval(endpoints.to_y, STUB_ROW_KEEPOUT_RADIUS));
            }

            if intervals.is_empty() {
                continue;
            }

            let intervals = merge_intervals(intervals);
            let mut min_y = usize::MAX;
            let mut max_y = 0usize;
            for (y0, y1) in &intervals {
                min_y = min_y.min(*y0);
                max_y = max_y.max(*y1);
            }
            usages.push(EdgeGapUsage {
                edge_idx,
                min_y,
                max_y,
                intervals,
            });
        }

        if usages.is_empty() {
            continue;
        }

        usages.sort_by(|a, b| {
            a.min_y
                .cmp(&b.min_y)
                .then_with(|| a.max_y.cmp(&b.max_y))
                .then_with(|| a.edge_idx.cmp(&b.edge_idx))
        });

        let mut gap_width = gap_widths[gap_idx].max(min_gap_width);
        let max_gap_width = gap_width.saturating_add(usages.len().saturating_mul(6) + 16);

        let mut assigned = None::<(Vec<Option<usize>>, Vec<Vec<(usize, usize)>>)>;
        while gap_width <= max_gap_width {
            let candidates = gap_lane_x_candidates(0, gap_width);
            if candidates.is_empty() {
                gap_width = gap_width.saturating_add(1);
                continue;
            }

            let mut lane_occupied = vec![Vec::<(usize, usize)>::new(); candidates.len()];
            let mut lane_used = vec![false; candidates.len()];
            let mut lanes_for_edge = vec![None::<usize>; edge_count];
            let mut failed = false;

            for usage in &usages {
                let mut chosen_lane = None::<usize>;
                'candidate: for lane_idx in 0..candidates.len() {
                    if intervals_overlap(&usage.intervals, &lane_occupied[lane_idx]) {
                        continue;
                    }

                    let lane_x = candidates[lane_idx];
                    for other_lane in 0..candidates.len() {
                        if !lane_used[other_lane] {
                            continue;
                        }
                        if !intervals_overlap(&usage.intervals, &lane_occupied[other_lane]) {
                            continue;
                        }

                        let other_x = candidates[other_lane];
                        let distance = lane_x.abs_diff(other_x);
                        if distance < LANE_MIN_X_CLEARANCE {
                            continue 'candidate;
                        }
                    }

                    chosen_lane = Some(lane_idx);
                    break;
                }

                let Some(lane_idx) = chosen_lane else {
                    failed = true;
                    break;
                };

                lanes_for_edge[usage.edge_idx] = Some(lane_idx);
                lane_used[lane_idx] = true;
                let mut merged = lane_occupied[lane_idx].clone();
                merged.extend_from_slice(&usage.intervals);
                lane_occupied[lane_idx] = merge_intervals(merged);
            }

            if failed {
                gap_width = gap_width.saturating_add(1);
                continue;
            }

            if lane_used.iter().filter(|used| **used).count() > 1 {
                let mut lanes_by_y = lane_occupied
                    .iter()
                    .enumerate()
                    .filter_map(|(lane_idx, intervals)| {
                        if intervals.is_empty() {
                            return None;
                        }
                        let min_y = intervals
                            .iter()
                            .map(|(y0, _y1)| *y0)
                            .min()
                            .unwrap_or(usize::MAX);
                        Some((min_y, lane_idx))
                    })
                    .collect::<Vec<_>>();

                // Place lanes with lower min_y (visually higher) further to the right so stubs
                // from lower nodes are less likely to bridge across them.
                lanes_by_y.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

                let mut lanes_by_x = candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(lane_idx, x)| lane_used[lane_idx].then_some((*x, lane_idx)))
                    .collect::<Vec<_>>();
                lanes_by_x.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

                let mut remap = (0..candidates.len()).collect::<Vec<_>>();
                for (rank, (_min_y, lane_idx)) in lanes_by_y.into_iter().enumerate() {
                    if let Some((_, target_lane)) = lanes_by_x.get(rank) {
                        remap[lane_idx] = *target_lane;
                    }
                }

                for lane in &mut lanes_for_edge {
                    if let Some(idx) = *lane {
                        *lane = Some(remap[idx]);
                    }
                }
            }

            assigned = Some((lanes_for_edge, lane_occupied));
            break;
        }

        let Some((lanes_for_edge, _lane_occupied)) = assigned else {
            continue;
        };

        for (edge_idx, lane_idx) in lanes_for_edge.into_iter().enumerate() {
            edge_gap_lanes[edge_idx][gap_idx] = lane_idx;
        }

        gap_widths[gap_idx] = gap_width.max(min_gap_width);
    }

    (edge_gap_lanes, gap_widths)
}

fn expand_row_interval(row: usize, radius: usize) -> (usize, usize) {
    (row.saturating_sub(radius), row.saturating_add(radius))
}

fn merge_intervals(mut intervals: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    intervals.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut out = Vec::<(usize, usize)>::new();

    for (start, end) in intervals {
        if let Some((_, last_end)) = out.last_mut() {
            if start <= last_end.saturating_add(1) {
                *last_end = (*last_end).max(end);
                continue;
            }
        }
        out.push((start, end));
    }

    out
}

fn intervals_overlap(a: &[(usize, usize)], b: &[(usize, usize)]) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }

    let mut i = 0usize;
    let mut j = 0usize;

    while i < a.len() && j < b.len() {
        let (a0, a1) = a[i];
        let (b0, b1) = b[j];

        if a1 < b0 {
            i += 1;
            continue;
        }
        if b1 < a0 {
            j += 1;
            continue;
        }
        return true;
    }

    false
}

fn spans_to_cells(spans: &[LineSpan]) -> BTreeSet<(usize, usize)> {
    let mut cells = BTreeSet::new();
    for (y, x0, x1) in spans.iter().copied() {
        let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for x in min_x..=max_x {
            cells.insert((x, y));
        }
    }
    cells
}

fn cells_overlap_or_side_touch(
    a_cells: &BTreeSet<(usize, usize)>,
    b_cells: &BTreeSet<(usize, usize)>,
) -> bool {
    for &(x, y) in a_cells {
        if b_cells.contains(&(x, y)) {
            return true;
        }
        if let Some(nx) = x.checked_sub(1) {
            if b_cells.contains(&(nx, y)) {
                return true;
            }
        }
        if let Some(nx) = x.checked_add(1) {
            if b_cells.contains(&(nx, y)) {
                return true;
            }
        }
        if let Some(ny) = y.checked_sub(1) {
            if b_cells.contains(&(x, ny)) {
                return true;
            }
        }
        if let Some(ny) = y.checked_add(1) {
            if b_cells.contains(&(x, ny)) {
                return true;
            }
        }
    }
    false
}

fn has_non_endpoint_edge_touch(
    ast: &FlowchartAst,
    node_renders: &BTreeMap<ObjectId, NodeRender>,
    layer_metrics: &[LayerMetrics],
    gap_widths: &[usize],
    routes: &[Vec<GridPoint>],
    box_height: usize,
    edge_gap_lanes: &[Vec<Option<usize>>],
) -> Result<bool, FlowchartRenderError> {
    let width = layer_metrics.last().map(|layer| layer.x1 + 1).unwrap_or(1);
    let base_height = node_renders
        .values()
        .map(|render| render.box_y1.saturating_add(1))
        .max()
        .unwrap_or(1);
    let height = routed_height(base_height, routes, box_height);
    let vertical_occupied = connector_vertical_occupancy_mask(
        ast,
        layer_metrics,
        gap_widths,
        node_renders,
        routes,
        box_height,
        edge_gap_lanes,
        width,
        height,
    );

    let mut edge_cells = Vec::<BTreeSet<(usize, usize)>>::with_capacity(ast.edges().len());
    let mut edge_endpoints = Vec::<(ObjectId, ObjectId)>::with_capacity(ast.edges().len());

    for (edge_idx, (_edge_id, edge)) in ast.edges().iter().enumerate() {
        let from = node_renders
            .get(edge.from_node_id())
            .copied()
            .ok_or_else(|| FlowchartRenderError::MissingPlacement {
                node_id: edge.from_node_id().clone(),
            })?;
        let to = node_renders
            .get(edge.to_node_id())
            .copied()
            .ok_or_else(|| FlowchartRenderError::MissingPlacement {
                node_id: edge.to_node_id().clone(),
            })?;

        let spans = match routes.get(edge_idx) {
            Some(route) if route.len() >= 2 => routed_connector_spans_bridged(
                from,
                to,
                layer_metrics,
                gap_widths,
                route,
                box_height,
                edge_idx,
                edge_gap_lanes,
                &vertical_occupied,
                width,
            ),
            _ => connector_spans_bridged(from, to, &vertical_occupied, width),
        };

        edge_cells.push(spans_to_cells(&spans));
        edge_endpoints.push((edge.from_node_id().clone(), edge.to_node_id().clone()));
    }

    for i in 0..edge_cells.len() {
        for j in (i + 1)..edge_cells.len() {
            let (from_a, to_a) = &edge_endpoints[i];
            let (from_b, to_b) = &edge_endpoints[j];
            let shares_endpoint =
                from_a == from_b || from_a == to_b || to_a == from_b || to_a == to_b;
            if shares_endpoint {
                continue;
            }
            if cells_overlap_or_side_touch(&edge_cells[i], &edge_cells[j]) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn draw_connector_pass(
    canvas: &mut Canvas,
    from: NodeRender,
    to: NodeRender,
    pass: ConnectorDrawPass,
) -> Result<(), CanvasError> {
    let from_x = from.box_x1;
    let from_y = from.mid_y();
    let to_x = to.box_x0;
    let to_y = to.mid_y();

    if from_y == to_y {
        if pass == ConnectorDrawPass::Horizontal {
            draw_hline_bridge_vertical(canvas, from_x, to_x, from_y)?;
        }
        return Ok(());
    }

    let bend_x = (from_x + to_x) / 2;
    if pass == ConnectorDrawPass::Vertical {
        canvas.draw_vline(bend_x, from_y, to_y)?;
    } else {
        draw_hline_bridge_vertical(canvas, from_x, bend_x, from_y)?;
        draw_hline_bridge_vertical(canvas, bend_x, to_x, to_y)?;
    }
    Ok(())
}

fn draw_hline_bridge_vertical(
    canvas: &mut Canvas,
    x0: usize,
    x1: usize,
    y: usize,
) -> Result<(), CanvasError> {
    let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };

    if y >= canvas.height() {
        return Err(CanvasError::OutOfBounds {
            x: min_x,
            y,
            width: canvas.width(),
            height: canvas.height(),
        });
    }

    if max_x >= canvas.width() {
        return Err(CanvasError::OutOfBounds {
            x: max_x,
            y,
            width: canvas.width(),
            height: canvas.height(),
        });
    }

    for x in min_x..=max_x {
        let is_endpoint = x == min_x || x == max_x;
        if !is_endpoint && canvas.has_box_vertical(x, y)? {
            continue;
        }
        canvas.set(x, y, super::UNICODE_BOX_HORIZONTAL)?;
    }

    Ok(())
}

fn draw_routed_connector(
    canvas: &mut Canvas,
    from: NodeRender,
    to: NodeRender,
    layer_metrics: &[LayerMetrics],
    gap_widths: &[usize],
    route: &[GridPoint],
    box_height: usize,
    edge_idx: usize,
    edge_gap_lanes: &[Vec<Option<usize>>],
    pass: ConnectorDrawPass,
) -> Result<(), CanvasError> {
    if route.len() < 2 {
        return draw_connector_pass(canvas, from, to, pass);
    }

    let mut points = Vec::<(usize, usize)>::with_capacity(route.len());
    for (idx, p) in route.iter().enumerate() {
        let x = match route_grid_x_to_lane_x(
            route,
            idx,
            from,
            to,
            layer_metrics,
            gap_widths,
            edge_idx,
            edge_gap_lanes,
        ) {
            Some(x) => x,
            None => return draw_connector_pass(canvas, from, to, pass),
        };
        let y = grid_y_to_canvas_y(p.y(), box_height);
        let point = (x, y);
        if points.last() != Some(&point) {
            points.push(point);
        }
    }

    // Stubs connect the routed polyline (in lane space) to the node boxes.
    let (start_x, start_y) = points.first().copied().expect("non-empty");
    let from_y = from.mid_y();
    let from_x = if start_x >= from.box_x1 {
        from.box_x1
    } else {
        from.box_x0
    };

    let (end_x, end_y) = points.last().copied().expect("non-empty");
    let to_y = to.mid_y();
    let to_x = if end_x <= to.box_x0 {
        to.box_x0
    } else {
        to.box_x1
    };

    match pass {
        ConnectorDrawPass::Vertical => {
            for pair in points.windows(2) {
                let (x0, y0) = pair[0];
                let (x1, y1) = pair[1];
                if x0 == x1 && y0 == y1 {
                    continue;
                }
                if x0 == x1 {
                    canvas.draw_vline(x0, y0, y1)?;
                } else if y0 != y1 {
                    // Routing should be orthogonal; fall back to a deterministic L to avoid crashing.
                    canvas.draw_vline(x1, y0, y1)?;
                }
            }

            // Ensure the stub y matches the lane y (defensive: should already match for endpoints).
            if start_y != from_y {
                canvas.draw_vline(start_x, start_y, from_y)?;
            }
            if end_y != to_y {
                canvas.draw_vline(end_x, end_y, to_y)?;
            }
        }
        ConnectorDrawPass::Horizontal => {
            for pair in points.windows(2) {
                let (x0, y0) = pair[0];
                let (x1, y1) = pair[1];
                if x0 == x1 && y0 == y1 {
                    continue;
                }
                if y0 == y1 {
                    draw_hline_bridge_vertical(canvas, x0, x1, y0)?;
                } else if x0 != x1 {
                    // Routing should be orthogonal; fall back to a deterministic L to avoid crashing.
                    draw_hline_bridge_vertical(canvas, x0, x1, y0)?;
                }
            }

            draw_hline_bridge_vertical(canvas, from_x, start_x, from_y)?;
            draw_hline_bridge_vertical(canvas, end_x, to_x, to_y)?;
        }
    }

    Ok(())
}

fn route_grid_x_to_lane_x(
    route: &[GridPoint],
    idx: usize,
    from: NodeRender,
    to: NodeRender,
    layer_metrics: &[LayerMetrics],
    gap_widths: &[usize],
    edge_idx: usize,
    edge_gap_lanes: &[Vec<Option<usize>>],
) -> Option<usize> {
    let gap_idx = route_grid_x_to_lane_gap(route, idx, from.layer, to.layer, layer_metrics.len())?;
    lane_x_for_gap(gap_idx, edge_idx, layer_metrics, edge_gap_lanes, gap_widths)
}

fn route_grid_x_to_lane_gap(
    route: &[GridPoint],
    idx: usize,
    from_layer: usize,
    to_layer: usize,
    layer_count: usize,
) -> Option<usize> {
    let grid_x = route.get(idx)?.x();
    if grid_x < 0 {
        return None;
    }

    // Odd x are always "between layers": map to the lane in that gap.
    if grid_x % 2 != 0 {
        let between_layer: usize = ((grid_x - 1) / 2).try_into().ok()?;
        return gap_after_layer(between_layer, layer_count);
    }

    let layer: usize = (grid_x / 2).try_into().ok()?;
    if layer >= layer_count {
        return None;
    }

    // Even x are layer columns. Route segments should stay in lanes to avoid drawing through node
    // boxes. Pick the gap on the side where this grid point has a horizontal neighbor.
    let mut connects_left = false;
    let mut connects_right = false;
    if idx > 0 {
        let prev_x = route[idx - 1].x();
        connects_left |= prev_x < grid_x;
        connects_right |= prev_x > grid_x;
    }
    if idx + 1 < route.len() {
        let next_x = route[idx + 1].x();
        connects_left |= next_x < grid_x;
        connects_right |= next_x > grid_x;
    }

    if connects_left && !connects_right {
        return gap_before_layer(layer, layer_count)
            .or_else(|| gap_after_layer(layer, layer_count));
    }
    if connects_right && !connects_left {
        return gap_after_layer(layer, layer_count)
            .or_else(|| gap_before_layer(layer, layer_count));
    }

    // Ambiguous: this grid point has no immediate horizontal neighbor (often a vertical segment on a
    // layer column). Prefer the first horizontal turn direction in the route so vertical segments
    // don't "jump lanes" when projected into canvas space.
    let mut prefers_left = false;
    let mut prefers_right = false;

    if idx > 0 {
        for j in (0..idx).rev() {
            let x = route[j].x();
            if x == grid_x {
                continue;
            }
            if x < grid_x {
                prefers_left = true;
            } else {
                prefers_right = true;
            }
            break;
        }
    }

    for j in (idx + 1)..route.len() {
        let x = route[j].x();
        if x == grid_x {
            continue;
        }
        if x < grid_x {
            prefers_left = true;
        } else {
            prefers_right = true;
        }
        break;
    }

    if prefers_left && !prefers_right {
        return gap_before_layer(layer, layer_count)
            .or_else(|| gap_after_layer(layer, layer_count));
    }
    if prefers_right && !prefers_left {
        return gap_after_layer(layer, layer_count)
            .or_else(|| gap_before_layer(layer, layer_count));
    }

    // Still ambiguous (e.g. purely vertical routes): use outgoing for the source, incoming for the
    // target.
    if idx == 0 {
        return gap_after_layer(from_layer, layer_count)
            .or_else(|| gap_before_layer(from_layer, layer_count));
    }
    if idx + 1 == route.len() {
        return gap_before_layer(to_layer, layer_count)
            .or_else(|| gap_after_layer(to_layer, layer_count));
    }

    gap_before_layer(layer, layer_count).or_else(|| gap_after_layer(layer, layer_count))
}

fn gap_after_layer(layer: usize, layer_count: usize) -> Option<usize> {
    (layer + 1 < layer_count).then_some(layer)
}

fn gap_before_layer(layer: usize, _layer_count: usize) -> Option<usize> {
    (layer > 0).then_some(layer.saturating_sub(1))
}

fn gap_lane_x_candidates(layer_x1: usize, gap_width: usize) -> Vec<usize> {
    if gap_width == 0 {
        return Vec::new();
    }

    let start_x = layer_x1 + 1;
    let end_x = layer_x1 + gap_width;
    let center_x = layer_x1 + (gap_width / 2);

    let mut out = Vec::<usize>::with_capacity(gap_width);
    if center_x >= start_x && center_x <= end_x {
        out.push(center_x);
    }

    for offset in 1..=gap_width {
        let right = center_x.saturating_add(offset);
        if right >= start_x && right <= end_x {
            out.push(right);
        }
        let left = center_x.saturating_sub(offset);
        if left >= start_x && left <= end_x {
            out.push(left);
        }
        if out.len() >= gap_width {
            break;
        }
    }

    out.truncate(gap_width);
    debug_assert_eq!(out.len(), gap_width);
    out
}

fn lane_x_for_gap(
    gap_idx: usize,
    edge_idx: usize,
    layer_metrics: &[LayerMetrics],
    edge_gap_lanes: &[Vec<Option<usize>>],
    gap_widths: &[usize],
) -> Option<usize> {
    if gap_idx + 1 >= layer_metrics.len() {
        return None;
    }
    let layer = layer_metrics.get(gap_idx)?;
    let gap_width = gap_widths.get(gap_idx).copied().unwrap_or(MIN_COL_GAP);
    let candidates = gap_lane_x_candidates(layer.x1, gap_width);
    if candidates.is_empty() {
        return None;
    }
    let lane_idx = edge_gap_lanes
        .get(edge_idx)
        .and_then(|lanes| lanes.get(gap_idx))
        .copied()
        .flatten();
    let lane_x = lane_idx
        .and_then(|idx| candidates.get(idx).copied())
        .unwrap_or_else(|| candidates[0]);
    Some(lane_x)
}

fn grid_y_to_canvas_y(grid_y: i32, box_height: usize) -> usize {
    let stride = box_height + ROW_GAP;

    if grid_y % 2 == 0 {
        let row: usize = (grid_y / 2).try_into().unwrap_or(0);
        return (row * stride) + 1;
    }

    let gap_idx: usize = ((grid_y - 1) / 2).try_into().unwrap_or(0);
    (gap_idx * stride) + box_height + (ROW_GAP / 2)
}

fn flow_box_height(options: RenderOptions) -> usize {
    if options.show_notes {
        BOX_HEIGHT_WITH_NOTES
    } else {
        BOX_HEIGHT_NO_NOTES
    }
}
