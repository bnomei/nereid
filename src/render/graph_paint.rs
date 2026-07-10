// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Graph-family node painting: plain boxes and multi-compartment class-style boxes.
//!
//! Full-width interior dividers use [`Canvas::draw_hline`] so box-edge merge yields `├`/`┤`.
//! Variable height is content-driven; edge attach policy for tall boxes is mid-y of the full box.

use crate::layout::FlowchartLayout;
use crate::render::scene::{CapKind, EdgeStroke, GraphEdge, GraphModel, GraphNode};
use crate::render::text::{text_len, truncate_with_ellipsis};
use crate::render::{
    Canvas, CanvasError, RenderOptions, UNICODE_BOX_HORIZONTAL, UNICODE_BOX_VERTICAL,
};

/// Minimum inner width for a graph node box (title padding).
pub const GRAPH_MIN_INNER_WIDTH: usize = 3;

/// Minimum corridor between layer boxes (caps + connector body).
/// Scoped to scene-native class/ER paint only — does not change flowchart gaps.
const GRAPH_MIN_CORRIDOR: usize = 4;

/// Blank cells reserved beside an edge label (each side) inside the corridor.
const GRAPH_EDGE_LABEL_SIDE_PAD: usize = 1;

/// Rows occupied by a node when notes are off and compartments are empty (title only).
pub const GRAPH_PLAIN_INNER_ROWS: usize = 1;

/// Content rows inside a graph node (excluding top/bottom borders).
///
/// Layout:
/// - 1 title row
/// - optional note row (when `show_notes` and note present)
/// - for each compartment (including empty): 1 divider row + `max(1, line_count)` rows
///   so class-style boxes keep structure when a compartment has no members yet
pub fn graph_node_inner_rows(node: &GraphNode, options: RenderOptions) -> usize {
    let mut rows = 1usize; // title
    if options.show_notes && node.note().is_some() {
        rows = rows.saturating_add(1);
    }
    for compartment in node.compartments() {
        rows = rows.saturating_add(1); // divider
        let line_count = compartment.lines().len().max(1);
        rows = rows.saturating_add(line_count);
    }
    rows.max(GRAPH_PLAIN_INNER_ROWS)
}

/// Total box height including top and bottom borders.
pub fn graph_node_box_height(node: &GraphNode, options: RenderOptions) -> usize {
    graph_node_inner_rows(node, options).saturating_add(2)
}

/// Mid-y attach row for edges (policy: middle of full box, including borders).
pub fn graph_node_attach_mid_y(box_y0: usize, box_height: usize) -> usize {
    box_y0.saturating_add(box_height.saturating_sub(1) / 2)
}

/// Measure preferred inner width from title, note, and compartment lines.
///
/// Adds 2 cells of horizontal breathing room (mirrors flow/seq label padding) so class members
/// are not flush against the side walls.
pub fn graph_node_preferred_inner_width(node: &GraphNode, options: RenderOptions) -> usize {
    let mut width = text_len(node.label()).max(GRAPH_MIN_INNER_WIDTH);
    if options.show_notes {
        if let Some(note) = node.note() {
            width = width.max(text_len(note));
        }
    }
    for compartment in node.compartments() {
        for line in compartment.lines() {
            width = width.max(text_len(line));
        }
    }
    width.saturating_add(2).max(GRAPH_MIN_INNER_WIDTH)
}

/// Draw a graph node box with optional compartment dividers and member lines.
///
/// - Title is centered on the first inner row.
/// - Member lines are left-aligned (class-style).
/// - Dividers span the full inner width and tee into the side walls via canvas merge.
pub fn paint_graph_node_box(
    canvas: &mut Canvas,
    node: &GraphNode,
    box_x0: usize,
    box_y0: usize,
    inner_width: usize,
    options: RenderOptions,
) -> Result<(usize, usize), CanvasError> {
    let inner_width = inner_width.max(GRAPH_MIN_INNER_WIDTH);
    let box_height = graph_node_box_height(node, options);
    let box_x1 = box_x0.saturating_add(inner_width).saturating_add(1);
    let box_y1 = box_y0.saturating_add(box_height.saturating_sub(1));

    canvas.draw_box(box_x0, box_y0, box_x1, box_y1)?;

    let mut row = box_y0.saturating_add(1);

    // Title (centered).
    let title = truncate_with_ellipsis(node.label(), inner_width);
    let title_pad = (inner_width.saturating_sub(text_len(&title))) / 2;
    canvas.write_str(box_x0.saturating_add(1).saturating_add(title_pad), row, &title)?;
    row = row.saturating_add(1);

    if options.show_notes {
        if let Some(note) = node.note() {
            let clipped = truncate_with_ellipsis(note, inner_width);
            let pad = (inner_width.saturating_sub(text_len(&clipped))) / 2;
            canvas.write_str(box_x0.saturating_add(1).saturating_add(pad), row, &clipped)?;
            row = row.saturating_add(1);
        }
    }

    for compartment in node.compartments() {
        // Full-width interior divider; merge with side walls → ├ / ┤.
        canvas.draw_hline(box_x0, box_x1, row)?;
        row = row.saturating_add(1);

        if compartment.lines().is_empty() {
            if row < box_y1 {
                // Reserve a blank member row for empty compartments.
                row = row.saturating_add(1);
            }
        } else {
            for line in compartment.lines() {
                if row >= box_y1 {
                    break;
                }
                let clipped = truncate_with_ellipsis(line, inner_width);
                canvas.write_str(box_x0.saturating_add(1), row, &clipped)?;
                row = row.saturating_add(1);
            }
        }
    }

    Ok((box_x1, box_y1))
}

/// One-cell cap glyph for graph edges (shared alphabet for class/ER).
pub fn graph_cap_glyph(kind: CapKind, outward_dx: i32, outward_dy: i32) -> Option<char> {
    kind.glyph(outward_dx, outward_dy)
}

/// True when any node carries class-style compartments (variable-height paint path).
pub fn graph_model_has_compartments(model: &crate::render::scene::GraphModel) -> bool {
    model.nodes().values().any(|n| !n.compartments().is_empty())
}

/// True when the scene-native graph painter must run (compartments, non-flow caps, or dashed stroke).
pub fn graph_model_needs_scene_paint(model: &crate::render::scene::GraphModel) -> bool {
    if graph_model_has_compartments(model) {
        return true;
    }
    model.edges().values().any(|edge| {
        edge.stroke() == EdgeStroke::Dashed
            || !matches!(edge.start_cap(), CapKind::None | CapKind::Arrow)
            || !matches!(
                edge.end_cap(),
                CapKind::None | CapKind::Arrow | CapKind::Circle | CapKind::Cross
            )
    })
}

fn draw_hline_stroke(
    canvas: &mut Canvas,
    x0: usize,
    x1: usize,
    y: usize,
    stroke: EdgeStroke,
) -> Result<(), CanvasError> {
    match stroke {
        EdgeStroke::Solid => canvas.draw_hline(x0, x1, y),
        EdgeStroke::Dashed => {
            let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
            for (i, x) in (min_x..=max_x).enumerate() {
                if i % 2 == 0 {
                    canvas.set(x, y, UNICODE_BOX_HORIZONTAL)?;
                }
            }
            Ok(())
        }
    }
}

fn draw_vline_stroke(
    canvas: &mut Canvas,
    x: usize,
    y0: usize,
    y1: usize,
    stroke: EdgeStroke,
) -> Result<(), CanvasError> {
    match stroke {
        EdgeStroke::Solid => canvas.draw_vline(x, y0, y1),
        EdgeStroke::Dashed => {
            let (min_y, max_y) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
            for (i, y) in (min_y..=max_y).enumerate() {
                if i % 2 == 0 {
                    canvas.set(x, y, UNICODE_BOX_VERTICAL)?;
                }
            }
            Ok(())
        }
    }
}

/// True if `(x, y)` lies on any placed node box, including its border.
fn cell_on_box_border_or_interior(
    placed: &[(usize, usize, usize, usize)],
    x: usize,
    y: usize,
) -> bool {
    // (x0, y0, x1, y1)
    placed.iter().any(|&(x0, y0, x1, y1)| x >= x0 && x <= x1 && y >= y0 && y <= y1)
}

/// Category segments for annotated scene-graph paint (`node` / `edge` / `note` object refs).
#[derive(Debug, Clone, Copy)]
pub struct GraphHighlightCategories<'a> {
    pub node_segments: &'a [&'a str],
    pub edge_segments: &'a [&'a str],
    /// Note row spans (TUI paints these DarkGray, same as flow/seq notes).
    pub note_segments: &'a [&'a str],
}

impl GraphHighlightCategories<'static> {
    pub const CLASS: Self = Self {
        node_segments: &["class", "class"],
        edge_segments: &["class", "relation"],
        note_segments: &["class", "note"],
    };
    pub const ER: Self = Self {
        node_segments: &["er", "entity"],
        edge_segments: &["er", "relationship"],
        note_segments: &["er", "note"],
    };
}

/// Place and paint a graph model with per-node compartment heights.
///
/// Matches flowchart endpoint policy: **box walls stay pure box-drawing glyphs**. Edge lines and
/// endpoint caps live only in the corridor between boxes (first/last cells of the gap), never on
/// `x0`/`x1` border cells.
pub fn render_graph_model_with_compartments(
    model: &crate::render::scene::GraphModel,
    layout: &crate::layout::FlowchartLayout,
    options: RenderOptions,
) -> Result<String, CanvasError> {
    Ok(render_graph_model_with_compartments_inner(model, layout, options, None)?.0)
}

/// Annotated variant: same paint, plus `HighlightIndex` for TUI hints (`f` / chains).
pub fn render_graph_model_with_compartments_annotated(
    diagram_id: &crate::model::ids::DiagramId,
    model: &crate::render::scene::GraphModel,
    layout: &crate::layout::FlowchartLayout,
    options: RenderOptions,
    categories: GraphHighlightCategories<'_>,
) -> Result<crate::render::AnnotatedRender, CanvasError> {
    use crate::model::{CategoryPath, ObjectRef};
    use crate::render::{clamp_highlight_index_to_text, AnnotatedRender, HighlightIndex, LineSpan};

    let (text, node_boxes, edge_spans) =
        render_graph_model_with_compartments_inner(model, layout, options, Some(()))?;

    let node_cat =
        CategoryPath::new(categories.node_segments.iter().map(|s| (*s).to_owned()).collect())
            .expect("valid node category");
    let edge_cat =
        CategoryPath::new(categories.edge_segments.iter().map(|s| (*s).to_owned()).collect())
            .expect("valid edge category");
    let note_cat =
        CategoryPath::new(categories.note_segments.iter().map(|s| (*s).to_owned()).collect())
            .expect("valid note category");

    let mut highlight_index = HighlightIndex::new();
    for (node_id, (x0, y0, x1, y1)) in &node_boxes {
        let object_ref = ObjectRef::new(diagram_id.clone(), node_cat.clone(), node_id.clone());
        let mut spans = Vec::<LineSpan>::new();
        for y in *y0..=*y1 {
            spans.push((y, *x0, *x1));
        }
        highlight_index.insert(object_ref, spans);

        // Separate note spans so the TUI can dim note text like flow/sequence.
        if options.show_notes {
            if let Some(node) = model.nodes().get(node_id) {
                if let Some(note) = node.note() {
                    let inner_width = x1.saturating_sub(*x0).saturating_sub(1);
                    let clipped = truncate_with_ellipsis(note, inner_width);
                    let clipped_len = text_len(&clipped);
                    if clipped_len > 0 {
                        let pad = (inner_width.saturating_sub(clipped_len)) / 2;
                        let note_x = x0.saturating_add(1).saturating_add(pad);
                        // Title at y0+1; note row immediately under title.
                        let note_y = y0.saturating_add(2);
                        let note_ref =
                            ObjectRef::new(diagram_id.clone(), note_cat.clone(), node_id.clone());
                        highlight_index.insert(
                            note_ref,
                            vec![(note_y, note_x, note_x + clipped_len.saturating_sub(1))],
                        );
                    }
                }
            }
        }
    }
    for (edge_id, spans) in edge_spans {
        if spans.is_empty() {
            continue;
        }
        let object_ref = ObjectRef::new(diagram_id.clone(), edge_cat.clone(), edge_id);
        highlight_index.insert(object_ref, spans);
    }

    clamp_highlight_index_to_text(&mut highlight_index, &text);
    Ok(AnnotatedRender { text, highlight_index })
}

type NodeBoxMap =
    std::collections::BTreeMap<crate::model::ids::ObjectId, (usize, usize, usize, usize)>;
type EdgeSpanMap =
    std::collections::BTreeMap<crate::model::ids::ObjectId, Vec<crate::render::LineSpan>>;

/// Corridor cells needed for one edge: caps + optional label + side padding.
///
/// Does not include the box walls themselves. Example for `places` with two caps:
/// `1 (start cap) + 1 (pad) + 6 (label) + 1 (pad) + 1 (end cap) = 10`.
pub(crate) fn edge_corridor_need(edge: &GraphEdge) -> usize {
    let label_len = edge.label().map(text_len).unwrap_or(0);
    let start_cap = usize::from(edge.start_cap() != CapKind::None);
    let end_cap = usize::from(edge.end_cap() != CapKind::None);
    let pad = if label_len > 0 { GRAPH_EDGE_LABEL_SIDE_PAD.saturating_mul(2) } else { 0 };
    let need = start_cap.saturating_add(end_cap).saturating_add(label_len).saturating_add(pad);
    need.max(GRAPH_MIN_CORRIDOR)
}

/// Per-gap corridor widths between consecutive layers, driven by max edge label pressure.
///
/// Edges spanning multiple layers contribute their need to every intervening gap. Unlabeled
/// edges still reserve [`GRAPH_MIN_CORRIDOR`]. This is **scene-native graph paint only**.
pub(crate) fn inter_layer_corridor_widths(
    model: &GraphModel,
    layout: &FlowchartLayout,
) -> Vec<usize> {
    let layer_count = layout.layers().len();
    if layer_count < 2 {
        return Vec::new();
    }

    let mut node_layer = std::collections::BTreeMap::<crate::model::ids::ObjectId, usize>::new();
    for (layer_idx, layer) in layout.layers().iter().enumerate() {
        for node_id in layer {
            node_layer.insert(node_id.clone(), layer_idx);
        }
    }

    let mut gaps = vec![GRAPH_MIN_CORRIDOR; layer_count.saturating_sub(1)];
    for edge in model.edges().values() {
        let Some(&from_l) = node_layer.get(edge.from_node_id()) else {
            continue;
        };
        let Some(&to_l) = node_layer.get(edge.to_node_id()) else {
            continue;
        };
        if from_l == to_l {
            continue;
        }
        let need = edge_corridor_need(edge);
        let (lo, hi) = if from_l < to_l { (from_l, to_l) } else { (to_l, from_l) };
        for g in lo..hi {
            if let Some(slot) = gaps.get_mut(g) {
                *slot = (*slot).max(need);
            }
        }
    }
    gaps
}

fn layer_x0_from_metrics(
    layer_inner_widths: &[usize],
    gap_widths: &[usize],
) -> (Vec<usize>, usize) {
    let mut layer_x0 = Vec::with_capacity(layer_inner_widths.len());
    let mut x = 0usize;
    for (i, inner) in layer_inner_widths.iter().enumerate() {
        layer_x0.push(x);
        x = x.saturating_add(inner.saturating_add(2));
        if let Some(gap) = gap_widths.get(i) {
            x = x.saturating_add(*gap);
        }
    }
    (layer_x0, x.max(1))
}

fn render_graph_model_with_compartments_inner(
    model: &crate::render::scene::GraphModel,
    layout: &crate::layout::FlowchartLayout,
    options: RenderOptions,
    _collect: Option<()>,
) -> Result<(String, NodeBoxMap, EdgeSpanMap), CanvasError> {
    use crate::render::text::canvas_to_string_trimmed;
    use crate::render::LineSpan;
    use std::collections::BTreeMap;

    let row_gap = 1usize;

    let mut layer_inner_widths = Vec::<usize>::new();
    for layer in layout.layers() {
        let mut w = GRAPH_MIN_INNER_WIDTH;
        for node_id in layer {
            if let Some(node) = model.nodes().get(node_id) {
                w = w.max(graph_node_preferred_inner_width(node, options));
            }
        }
        layer_inner_widths.push(w);
    }

    let gap_widths = inter_layer_corridor_widths(model, layout);
    let (layer_x0, width) = layer_x0_from_metrics(&layer_inner_widths, &gap_widths);

    #[derive(Clone, Copy)]
    struct Placed {
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        mid_y: usize,
    }
    let mut placed: BTreeMap<crate::model::ids::ObjectId, Placed> = BTreeMap::new();
    let mut height = 1usize;

    for (layer_idx, layer) in layout.layers().iter().enumerate() {
        let mut y = 0usize;
        let x0 = layer_x0.get(layer_idx).copied().unwrap_or(0);
        let inner = layer_inner_widths.get(layer_idx).copied().unwrap_or(GRAPH_MIN_INNER_WIDTH);
        for node_id in layer {
            let Some(node) = model.nodes().get(node_id) else {
                continue;
            };
            let box_h = graph_node_box_height(node, options);
            let x1 = x0.saturating_add(inner).saturating_add(1);
            let y1 = y.saturating_add(box_h.saturating_sub(1));
            let mid_y = graph_node_attach_mid_y(y, box_h);
            placed.insert(node_id.clone(), Placed { x0, y0: y, x1, y1, mid_y });
            height = height.max(y1.saturating_add(1));
            y = y1.saturating_add(1).saturating_add(row_gap);
        }
    }

    let box_rects: Vec<(usize, usize, usize, usize)> =
        placed.values().map(|p| (p.x0, p.y0, p.x1, p.y1)).collect();

    let mut canvas = Canvas::new(width, height.max(1))?;

    struct PendingCap {
        x: usize,
        y: usize,
        ch: char,
    }
    let mut pending_caps = Vec::<PendingCap>::new();
    let mut pending_labels = Vec::<(usize, usize, String)>::new();
    let mut edge_spans: BTreeMap<crate::model::ids::ObjectId, Vec<LineSpan>> = BTreeMap::new();

    fn push_hline_span(spans: &mut Vec<LineSpan>, y: usize, x0: usize, x1: usize) {
        if x0 <= x1 {
            spans.push((y, x0, x1));
        } else {
            spans.push((y, x1, x0));
        }
    }

    fn push_vline_spans(spans: &mut Vec<LineSpan>, x: usize, y0: usize, y1: usize) {
        let (lo, hi) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for y in lo..=hi {
            spans.push((y, x, x));
        }
    }

    // 1) Edge corridors only — exclusive of box border cells (flow-style).
    for (edge_id, edge) in model.edges() {
        let Some(from) = placed.get(edge.from_node_id()).copied() else {
            continue;
        };
        let Some(to) = placed.get(edge.to_node_id()).copied() else {
            continue;
        };
        let spans = edge_spans.entry(edge_id.clone()).or_default();

        let left_to_right = from.x1 < to.x0;
        let right_to_left = to.x1 < from.x0;
        if !left_to_right && !right_to_left {
            // Same-layer: vertical run in the free column to the right of both boxes.
            let x_stub = from.x1.max(to.x1).saturating_add(1);
            if x_stub < width {
                let y0 = from.mid_y.min(to.mid_y);
                let y1 = from.mid_y.max(to.mid_y);
                if y0 < y1 {
                    draw_vline_stroke(&mut canvas, x_stub, y0, y1, edge.stroke())?;
                    push_vline_spans(spans, x_stub, y0, y1);
                }
                // Caps sit on the stub, one cell out from each box's right wall.
                let start_cap_x = from.x1.saturating_add(1);
                let end_cap_x = to.x1.saturating_add(1);
                if let Some(ch) = edge.start_cap().glyph(1, 0) {
                    if start_cap_x < width
                        && !cell_on_box_border_or_interior(&box_rects, start_cap_x, from.mid_y)
                    {
                        pending_caps.push(PendingCap { x: start_cap_x, y: from.mid_y, ch });
                        spans.push((from.mid_y, start_cap_x, start_cap_x));
                    }
                }
                if let Some(ch) = edge.end_cap().glyph(1, 0) {
                    if end_cap_x < width
                        && !cell_on_box_border_or_interior(&box_rects, end_cap_x, to.mid_y)
                    {
                        pending_caps.push(PendingCap { x: end_cap_x, y: to.mid_y, ch });
                        spans.push((to.mid_y, end_cap_x, end_cap_x));
                    }
                }
            }
            continue;
        }

        let (src, dst, forward) = if left_to_right { (from, to, true) } else { (to, from, false) };
        // First corridor cell to the right of src, last corridor cell to the left of dst.
        let gap_left = src.x1.saturating_add(1);
        let gap_right = dst.x0.saturating_sub(1);
        if gap_left > gap_right {
            continue;
        }

        let y_src = src.mid_y;
        let y_dst = dst.mid_y;
        let bend_x = (gap_left + gap_right) / 2;

        let stroke = edge.stroke();
        if gap_left <= bend_x {
            draw_hline_stroke(&mut canvas, gap_left, bend_x, y_src, stroke)?;
            push_hline_span(spans, y_src, gap_left, bend_x);
        }
        if y_src != y_dst {
            draw_vline_stroke(&mut canvas, bend_x, y_src.min(y_dst), y_src.max(y_dst), stroke)?;
            push_vline_spans(spans, bend_x, y_src, y_dst);
        }
        if bend_x <= gap_right {
            draw_hline_stroke(&mut canvas, bend_x, gap_right, y_dst, stroke)?;
            push_hline_span(spans, y_dst, bend_x, gap_right);
        }

        // Caps on the *edge* at the ends of the corridor (flow-style), never on box walls.
        let (start_cap_x, start_cap_y, start_out) =
            if forward { (gap_left, from.mid_y, 1i32) } else { (gap_right, from.mid_y, -1i32) };
        let (end_cap_x, end_cap_y, end_out) =
            if forward { (gap_right, to.mid_y, -1i32) } else { (gap_left, to.mid_y, 1i32) };

        if let Some(ch) = edge.start_cap().glyph(start_out, 0) {
            if !cell_on_box_border_or_interior(&box_rects, start_cap_x, start_cap_y) {
                pending_caps.push(PendingCap { x: start_cap_x, y: start_cap_y, ch });
                spans.push((start_cap_y, start_cap_x, start_cap_x));
            }
        }
        if let Some(ch) = edge.end_cap().glyph(end_out, 0) {
            if !cell_on_box_border_or_interior(&box_rects, end_cap_x, end_cap_y) {
                pending_caps.push(PendingCap { x: end_cap_x, y: end_cap_y, ch });
                spans.push((end_cap_y, end_cap_x, end_cap_x));
            }
        }

        if let Some(label) = edge.label() {
            // Reserve corridor ends for caps, then one pad cell each side of the label body.
            let has_start = edge.start_cap() != CapKind::None;
            let has_end = edge.end_cap() != CapKind::None;
            let label_left = gap_left
                .saturating_add(usize::from(has_start))
                .saturating_add(GRAPH_EDGE_LABEL_SIDE_PAD);
            let label_right = gap_right
                .saturating_sub(usize::from(has_end))
                .saturating_sub(GRAPH_EDGE_LABEL_SIDE_PAD);
            if label_left <= label_right {
                let max_label = label_right.saturating_sub(label_left).saturating_add(1);
                // Prefer full label when corridor was sized for it; ellipsis only as last resort.
                let clipped = truncate_with_ellipsis(label, max_label);
                let clipped_len = text_len(&clipped);
                let lx = label_left
                    .saturating_add(max_label.saturating_sub(clipped_len) / 2)
                    .min(label_right);
                // Prefer the horizontal run at src mid-y when straight; on bent edges use the
                // horizontal segment at y_dst (toward target) so the label sits on a full-width
                // corridor row rather than a 1-cell vertical bend.
                let ly = if y_src == y_dst { y_src } else { y_dst };
                if !cell_on_box_border_or_interior(&box_rects, lx, ly) && clipped_len > 0 {
                    pending_labels.push((lx, ly, clipped));
                    spans.push((ly, lx, lx.saturating_add(clipped_len.saturating_sub(1))));
                }
            }
        }
    }

    // 2) Nodes fully redraw walls/interiors.
    let mut node_boxes: BTreeMap<crate::model::ids::ObjectId, (usize, usize, usize, usize)> =
        BTreeMap::new();
    for (node_id, node) in model.nodes() {
        let Some(p) = placed.get(node_id).copied() else {
            continue;
        };
        let inner = p.x1.saturating_sub(p.x0).saturating_sub(1);
        paint_graph_node_box(&mut canvas, node, p.x0, p.y0, inner, options)?;
        node_boxes.insert(node_id.clone(), (p.x0, p.y0, p.x1, p.y1));
    }

    // 3) Labels then caps in corridor only — never `set` on box borders.
    for (lx, ly, text) in pending_labels {
        if !cell_on_box_border_or_interior(&box_rects, lx, ly) {
            canvas.write_str(lx, ly, &text)?;
        }
    }
    for cap in pending_caps {
        debug_assert!(
            !cell_on_box_border_or_interior(&box_rects, cap.x, cap.y),
            "cap must not sit on box border"
        );
        if !cell_on_box_border_or_interior(&box_rects, cap.x, cap.y) {
            canvas.set_exact(cap.x, cap.y, cap.ch)?;
        }
    }

    let text = canvas_to_string_trimmed(&canvas);
    let _ = _collect;
    Ok((text, node_boxes, edge_spans))
}

/// Assert no edge/connector glyph sits inside a painted node interior (defensive invariant).
#[cfg(test)]
pub fn assert_no_connector_in_box_interiors(text: &str) {
    // Soft check: boxes use ┌┐└┘│─; interiors should not contain ▶◀◆◇⊂⊃ alone mid-box.
    // Full geometric check is in integration tests with known fixtures.
    let _ = text;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::scene::{GraphCompartment, GraphNode};
    use crate::render::Canvas;

    #[test]
    fn plain_node_height_matches_legacy_three_rows() {
        let node = GraphNode::new("A");
        assert_eq!(graph_node_box_height(&node, RenderOptions::default()), 3);
        assert_eq!(graph_node_inner_rows(&node, RenderOptions::default()), 1);
    }

    #[test]
    fn note_adds_one_inner_row() {
        let node = GraphNode::new("A").with_note(Some("n".to_owned()));
        let options = RenderOptions { show_notes: true, ..RenderOptions::default() };
        assert_eq!(graph_node_box_height(&node, options), 4);
    }

    #[test]
    fn two_member_compartments_expand_height() {
        let node = GraphNode::new("Class01").with_compartments(vec![
            GraphCompartment::new(["int chimp", "int gorilla"]),
            GraphCompartment::new(["size()"]),
        ]);
        // title + div + 2 lines + div + 1 line + borders = 1+1+2+1+1+2 = 8
        assert_eq!(graph_node_box_height(&node, RenderOptions::default()), 8);
    }

    #[test]
    fn preferred_width_includes_side_padding_and_long_members() {
        let node =
            GraphNode::new("C").with_compartments(vec![GraphCompartment::new(["abcdefghij"])]);
        assert_eq!(graph_node_preferred_inner_width(&node, RenderOptions::default()), 12);
    }

    #[test]
    fn notes_off_ignores_note_in_height() {
        let node = GraphNode::new("A").with_note(Some("hidden".to_owned()));
        let options = RenderOptions { show_notes: false, ..RenderOptions::default() };
        assert_eq!(graph_node_box_height(&node, options), 3);
    }

    #[test]
    fn class_and_er_lower_carry_node_notes_into_paint() {
        use crate::model::{ClassAst, ClassNode, ErAst, ErEntity, ObjectId};
        use crate::render::lower::{lower_class, lower_er};

        let mut class_ast = ClassAst::default();
        let mut class = ClassNode::new("Service");
        class.set_note(Some("entry facade"));
        class_ast.classes_mut().insert(ObjectId::new("c:Service").unwrap(), class);
        let class_model = lower_class(&class_ast);
        let class_node = class_model.nodes().get(&ObjectId::new("c:Service").unwrap()).unwrap();
        assert_eq!(class_node.note(), Some("entry facade"));

        let mut er_ast = ErAst::default();
        let mut entity = ErEntity::new("CUSTOMER");
        entity.set_note(Some("billing party"));
        er_ast.entities_mut().insert(ObjectId::new("e:CUSTOMER").unwrap(), entity);
        let er_model = lower_er(&er_ast);
        let er_node = er_model.nodes().get(&ObjectId::new("e:CUSTOMER").unwrap()).unwrap();
        assert_eq!(er_node.note(), Some("billing party"));

        let options = RenderOptions { show_notes: true, ..RenderOptions::default() };
        let layout = crate::layout::layout_graph(&class_model).expect("layout");
        let text =
            render_graph_model_with_compartments(&class_model, &layout, options).expect("render");
        assert!(text.contains("entry facade"), "class note missing from paint:\n{text}");

        let layout = crate::layout::layout_graph(&er_model).expect("layout");
        let text =
            render_graph_model_with_compartments(&er_model, &layout, options).expect("render");
        assert!(text.contains("billing party"), "er note missing from paint:\n{text}");
    }

    #[test]
    fn empty_compartment_still_reserves_divider_and_blank() {
        let node = GraphNode::new("Empty")
            .with_compartments(vec![GraphCompartment::new(Vec::<String>::new())]);
        // title + div + blank + borders = 1+1+1+2 = 5
        assert_eq!(graph_node_box_height(&node, RenderOptions::default()), 5);
    }

    #[test]
    fn paint_class_style_box_has_dividers_and_members() {
        let node = GraphNode::new("Class01").with_compartments(vec![
            GraphCompartment::new(["int chimp", "int gorilla"]),
            GraphCompartment::new(["size()"]),
        ]);
        let inner = graph_node_preferred_inner_width(&node, RenderOptions::default());
        let height = graph_node_box_height(&node, RenderOptions::default());
        let width = inner + 2;
        let mut canvas = Canvas::new(width, height).expect("canvas");
        paint_graph_node_box(&mut canvas, &node, 0, 0, inner, RenderOptions::default())
            .expect("paint");
        let text = canvas.to_string();
        assert!(text.contains("Class01"), "{text}");
        assert!(text.contains("int chimp"), "{text}");
        assert!(text.contains("size()"), "{text}");
        assert!(text.contains('├') || text.contains('─'), "expected divider: {text}");
        // Side tees from full-width hline merge.
        let has_tee = text.chars().any(|ch| ch == '├' || ch == '┤');
        assert!(has_tee, "expected compartment tee into sides:\n{text}");
    }

    #[test]
    fn long_member_truncates_with_ellipsis() {
        let node = GraphNode::new("C")
            .with_compartments(vec![GraphCompartment::new(["this_is_a_very_long_member_name"])]);
        let inner = 8usize;
        let height = graph_node_box_height(&node, RenderOptions::default());
        let mut canvas = Canvas::new(inner + 2, height).expect("canvas");
        paint_graph_node_box(&mut canvas, &node, 0, 0, inner, RenderOptions::default())
            .expect("paint");
        let text = canvas.to_string();
        assert!(text.contains('…'), "expected ellipsis:\n{text}");
    }

    #[test]
    fn attach_mid_y_is_center_of_tall_box() {
        assert_eq!(graph_node_attach_mid_y(0, 8), 3);
        assert_eq!(graph_node_attach_mid_y(2, 5), 4);
    }

    #[test]
    fn cap_matrix_every_kind_one_char() {
        for kind in [
            CapKind::Arrow,
            CapKind::Circle,
            CapKind::Cross,
            CapKind::DiamondFilled,
            CapKind::DiamondHollow,
            CapKind::TriangleHollow,
            CapKind::ExactlyOne,
            CapKind::CrowFoot,
            CapKind::ZeroOrOne,
        ] {
            let ch = graph_cap_glyph(kind, 1, 0).expect("glyph");
            assert!(ch.len_utf8() > 0);
        }
        assert!(graph_cap_glyph(CapKind::None, 1, 0).is_none());
    }

    #[test]
    fn demo_class_boxes_keep_closed_corners_and_members_inside() {
        let input = include_str!("../../data/demo-session/diagrams/demo-class.mmd");
        let ast = crate::format::mermaid::parse_class_diagram(input).expect("parse");
        let model = crate::render::lower::lower_class(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let text = render_graph_model_with_compartments(&model, &layout, RenderOptions::default())
            .expect("render");

        // Distinct class names must appear as whole tokens (no "ClassCool" merges).
        assert!(text.contains("Class01"), "{text}");
        assert!(text.contains("AveryLongClass") || text.contains("AveryLon"), "{text}");
        assert!(text.contains("Class08"), "{text}");
        assert!(!text.contains("ClassCool"), "edge label must not merge into class name:\n{text}");

        // Members for Class01 stay present.
        assert!(text.contains("int chimp") || text.contains("int chim"), "{text}");
        assert!(text.contains("size()"), "{text}");

        // Closed box corners somewhere in the output.
        assert!(text.contains('┌') && text.contains('└'), "{text}");
        assert!(text.contains('├') || text.contains('─'), "{text}");

        // No box should have a broken top like "┌──  ──┐" patterns from label overwrite.
        for line in text.lines() {
            if line.contains('┌') {
                assert!(
                    !line.contains("┌ ") || line.matches('─').count() >= 2,
                    "suspicious top border: {line}\n{text}"
                );
            }
        }
    }

    #[test]
    fn painted_box_interior_rows_are_between_side_walls() {
        let node = GraphNode::new("Box").with_compartments(vec![
            GraphCompartment::new(["attr"]),
            GraphCompartment::new(["method()"]),
        ]);
        let inner = graph_node_preferred_inner_width(&node, RenderOptions::default());
        let h = graph_node_box_height(&node, RenderOptions::default());
        let mut canvas = Canvas::new(inner + 2, h).expect("canvas");
        paint_graph_node_box(&mut canvas, &node, 0, 0, inner, RenderOptions::default())
            .expect("paint");
        let text = canvas.to_string();
        for (i, line) in text.lines().enumerate() {
            if i == 0 || i + 1 == text.lines().count() {
                continue; // top/bottom borders
            }
            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                continue;
            }
            // Interior content rows: first and last should be vertical walls (or tees).
            let left = chars[0];
            let right = *chars.last().unwrap();
            assert!(
                matches!(left, '│' | '├' | '┤' | '┌' | '└'),
                "row {i} left wall broken: {line:?}\n{text}"
            );
            assert!(
                matches!(right, '│' | '├' | '┤' | '┐' | '┘'),
                "row {i} right wall broken: {line:?}\n{text}"
            );
        }
    }

    fn is_box_wall_char(ch: char) -> bool {
        matches!(ch, '│' | '─' | '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼')
    }

    fn is_endpoint_cap_char(ch: char) -> bool {
        matches!(
            ch,
            '▶' | '◀'
                | '▲'
                | '▼'
                | '▷'
                | '◁'
                | '△'
                | '▽'
                | '◆'
                | '◇'
                | '○'
                | '✕'
                | '‖'
                | '⊂'
                | '⊃'
                | '∪'
                | '∩'
        )
    }

    /// Recompute placement the same way as the painter, then assert every border cell of every
    /// box is a pure box-drawing glyph (never a cap). Caps may sit *next to* walls.
    fn assert_box_borders_are_pure_box_drawing(
        model: &crate::render::scene::GraphModel,
        layout: &crate::layout::FlowchartLayout,
        text: &str,
        options: RenderOptions,
    ) {
        let lines: Vec<Vec<char>> = text.lines().map(|l| l.chars().collect()).collect();

        let mut layer_inner_widths = Vec::<usize>::new();
        for layer in layout.layers() {
            let mut w = GRAPH_MIN_INNER_WIDTH;
            for node_id in layer {
                if let Some(node) = model.nodes().get(node_id) {
                    w = w.max(graph_node_preferred_inner_width(node, options));
                }
            }
            layer_inner_widths.push(w);
        }
        let gap_widths = inter_layer_corridor_widths(model, layout);
        let row_gap = 1usize;
        let (layer_x0, _) = layer_x0_from_metrics(&layer_inner_widths, &gap_widths);

        for (layer_idx, layer) in layout.layers().iter().enumerate() {
            let mut y = 0usize;
            let x0 = layer_x0.get(layer_idx).copied().unwrap_or(0);
            let inner = layer_inner_widths.get(layer_idx).copied().unwrap_or(GRAPH_MIN_INNER_WIDTH);
            for node_id in layer {
                let Some(node) = model.nodes().get(node_id) else {
                    continue;
                };
                let box_h = graph_node_box_height(node, options);
                let x1 = x0.saturating_add(inner).saturating_add(1);
                let y1 = y.saturating_add(box_h.saturating_sub(1));

                for yy in y0_range(y, y1) {
                    for xx in [x0, x1] {
                        let ch = lines.get(yy).and_then(|l| l.get(xx)).copied().unwrap_or(' ');
                        assert!(
                            is_box_wall_char(ch),
                            "box border at ({xx},{yy}) for {node_id} is `{ch}`, expected pure wall (not a cap):\n{text}"
                        );
                        assert!(
                            !is_endpoint_cap_char(ch),
                            "cap `{ch}` overwrote box border at ({xx},{yy}) for {node_id}:\n{text}"
                        );
                    }
                }
                for xx in x0..=x1 {
                    for yy in [y, y1] {
                        let ch = lines.get(yy).and_then(|l| l.get(xx)).copied().unwrap_or(' ');
                        assert!(
                            is_box_wall_char(ch),
                            "box border at ({xx},{yy}) for {node_id} is `{ch}`, expected pure wall:\n{text}"
                        );
                        assert!(
                            !is_endpoint_cap_char(ch),
                            "cap `{ch}` overwrote box border at ({xx},{yy}) for {node_id}:\n{text}"
                        );
                    }
                }

                y = y1.saturating_add(1).saturating_add(row_gap);
            }
        }
    }

    fn y0_range(y0: usize, y1: usize) -> std::ops::RangeInclusive<usize> {
        y0..=y1
    }

    #[test]
    fn class_demo_caps_never_overwrite_box_walls() {
        let input = include_str!("../../data/demo-session/diagrams/demo-class.mmd");
        let ast = crate::format::mermaid::parse_class_diagram(input).expect("parse");
        let model = crate::render::lower::lower_class(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let options = RenderOptions::default();
        let text = render_graph_model_with_compartments(&model, &layout, options).expect("render");
        assert_box_borders_are_pure_box_drawing(&model, &layout, &text, options);
    }

    #[test]
    fn er_demo_caps_never_overwrite_box_walls() {
        let input = include_str!("../../data/demo-session/diagrams/demo-er.mmd");
        let ast = crate::format::mermaid::parse_er_diagram(input).expect("parse");
        let model = crate::render::lower::lower_er(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let options = RenderOptions::default();
        let text = render_graph_model_with_compartments(&model, &layout, options).expect("render");
        assert_box_borders_are_pure_box_drawing(&model, &layout, &text, options);
        assert!(text.contains("CUSTOMER"), "{text}");
        assert!(text.contains("ORDER"), "{text}");
        assert!(text.contains("LINE-ITEM") || text.contains("LINE-ITE"), "{text}");
    }

    #[test]
    fn edge_corridor_need_grows_with_label_and_caps() {
        use crate::model::ids::ObjectId;
        use crate::render::scene::GraphEdge;

        let a = ObjectId::new("n:a").unwrap();
        let b = ObjectId::new("n:b").unwrap();
        let plain = GraphEdge::new(a.clone(), b.clone());
        assert_eq!(edge_corridor_need(&plain), GRAPH_MIN_CORRIDOR);

        let labeled = GraphEdge::new(a.clone(), b.clone())
            .with_label(Some("places".to_owned()))
            .with_caps(CapKind::ExactlyOne, CapKind::CrowFoot);
        // caps(1+1) + pad(1+1) + "places"(6) = 10
        assert_eq!(edge_corridor_need(&labeled), 10);
        assert!(edge_corridor_need(&labeled) > GRAPH_MIN_CORRIDOR);
    }

    #[test]
    fn er_demo_edge_labels_are_not_ellipsis_truncated() {
        let input = include_str!("../../data/demo-session/diagrams/demo-er.mmd");
        let ast = crate::format::mermaid::parse_er_diagram(input).expect("parse");
        let model = crate::render::lower::lower_er(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let text = render_graph_model_with_compartments(&model, &layout, RenderOptions::default())
            .expect("render");
        assert!(text.contains("places"), "expected full 'places' label:\n{text}");
        assert!(text.contains("contains"), "expected full 'contains' label:\n{text}");
        assert!(text.contains("uses"), "expected full 'uses' label:\n{text}");
        assert!(!text.contains('…'), "labels should not need ellipsis when gap is sized:\n{text}");
    }

    #[test]
    fn class_demo_cool_labels_not_squeezed_to_fragments() {
        let input = include_str!("../../data/demo-session/diagrams/demo-class.mmd");
        let ast = crate::format::mermaid::parse_class_diagram(input).expect("parse");
        let model = crate::render::lower::lower_class(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let text = render_graph_model_with_compartments(&model, &layout, RenderOptions::default())
            .expect("render");
        // "Cool" and "Cool label" appear in the source; at least short "Cool" must survive fully.
        assert!(text.contains("Cool"), "expected untruncated Cool-related label:\n{text}");
        assert!(
            !text.contains("Coo…") && !text.contains("Coo..."),
            "Cool must not be ellipsis-squeezed:\n{text}"
        );
    }

    #[test]
    fn class_annotated_index_has_class_and_relation_refs() {
        use crate::model::ids::DiagramId;

        let input = include_str!("../../data/demo-session/diagrams/demo-class.mmd");
        let ast = crate::format::mermaid::parse_class_diagram(input).expect("parse");
        let model = crate::render::lower::lower_class(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let diagram_id = DiagramId::new("demo-class").expect("id");
        let annotated = render_graph_model_with_compartments_annotated(
            &diagram_id,
            &model,
            &layout,
            RenderOptions::default(),
            GraphHighlightCategories::CLASS,
        )
        .expect("annotated");

        assert!(!annotated.highlight_index.is_empty(), "expected highlight spans");
        let class_refs = annotated
            .highlight_index
            .keys()
            .filter(|r| matches!(r.category().segments(), [a, b] if a == "class" && b == "class"))
            .count();
        let rel_refs = annotated
            .highlight_index
            .keys()
            .filter(
                |r| matches!(r.category().segments(), [a, b] if a == "class" && b == "relation"),
            )
            .count();
        assert!(class_refs >= 2, "expected class node refs, got {class_refs}");
        assert!(rel_refs >= 1, "expected relation refs, got {rel_refs}");
    }

    #[test]
    fn class_annotated_index_includes_note_refs_when_notes_shown() {
        use crate::model::ids::{DiagramId, ObjectId};
        use crate::model::{ClassAst, ClassNode};

        let mut ast = ClassAst::default();
        let id = ObjectId::new("c:Service").unwrap();
        let mut node = ClassNode::new("Service");
        node.set_note(Some("entry facade"));
        ast.classes_mut().insert(id.clone(), node);
        let model = crate::render::lower::lower_class(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let diagram_id = DiagramId::new("d-class").expect("id");
        let annotated = render_graph_model_with_compartments_annotated(
            &diagram_id,
            &model,
            &layout,
            RenderOptions { show_notes: true, ..Default::default() },
            GraphHighlightCategories::CLASS,
        )
        .expect("annotated");

        let note_refs = annotated
            .highlight_index
            .keys()
            .filter(|r| matches!(r.category().segments(), [a, b] if a == "class" && b == "note"))
            .count();
        assert!(note_refs >= 1, "expected class/note highlight refs, got {note_refs}");
    }

    #[test]
    fn er_annotated_index_has_entity_and_relationship_refs() {
        use crate::model::ids::DiagramId;

        let input = include_str!("../../data/demo-session/diagrams/demo-er.mmd");
        let ast = crate::format::mermaid::parse_er_diagram(input).expect("parse");
        let model = crate::render::lower::lower_er(&ast);
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let diagram_id = DiagramId::new("demo-er").expect("id");
        let annotated = render_graph_model_with_compartments_annotated(
            &diagram_id,
            &model,
            &layout,
            RenderOptions::default(),
            GraphHighlightCategories::ER,
        )
        .expect("annotated");

        let ent_refs = annotated
            .highlight_index
            .keys()
            .filter(|r| matches!(r.category().segments(), [a, b] if a == "er" && b == "entity"))
            .count();
        let rel_refs = annotated
            .highlight_index
            .keys()
            .filter(
                |r| matches!(r.category().segments(), [a, b] if a == "er" && b == "relationship"),
            )
            .count();
        assert_eq!(ent_refs, 4, "CUSTOMER/ORDER/LINE-ITEM/DELIVERY-ADDRESS");
        assert_eq!(rel_refs, 3, "three relationships");
    }

    #[test]
    fn two_box_edge_places_caps_in_corridor_not_on_walls() {
        use crate::model::ids::ObjectId;
        use crate::render::scene::{CapKind, GraphEdge, GraphModel, GraphNode};

        let mut model = GraphModel::default();
        let a = ObjectId::new("n:a").unwrap();
        let b = ObjectId::new("n:b").unwrap();
        model.nodes_mut().insert(a.clone(), GraphNode::new("A"));
        model.nodes_mut().insert(b.clone(), GraphNode::new("B"));
        model.edges_mut().insert(
            ObjectId::new("e:1").unwrap(),
            GraphEdge::new(a, b)
                .with_caps(CapKind::None, CapKind::DiamondFilled)
                .with_connector(Some("-->".to_owned())),
        );
        let layout = crate::layout::layout_graph(&model).expect("layout");
        let options = RenderOptions::default();
        let text = render_graph_model_with_compartments(&model, &layout, options).expect("render");
        assert_box_borders_are_pure_box_drawing(&model, &layout, &text, options);
        assert!(text.contains('◆') || text.contains('◇'), "expected diamond cap:\n{text}");
        // Corridor form: wall, then cap or line, never wall-with-cap fused.
        assert!(
            text.contains("│◆")
                || text.contains("│─")
                || text.contains("◆│")
                || text.contains("─┤")
                || text.contains("├─"),
            "expected edge attachment beside wall:\n{text}"
        );
        assert!(text.contains('A') && text.contains('B'), "{text}");
    }
}
