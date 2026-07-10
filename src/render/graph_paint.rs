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

use crate::render::scene::{CapKind, GraphNode};
use crate::render::text::{text_len, truncate_with_ellipsis};
use crate::render::{Canvas, CanvasError, RenderOptions};

/// Minimum inner width for a graph node box (title padding).
pub const GRAPH_MIN_INNER_WIDTH: usize = 3;

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

/// Place and paint a graph model with per-node compartment heights.
///
/// Uses flowchart layer order for columns; stacks nodes within a layer by content height.
/// Edges are straight mid-y stubs with endpoint caps (orthogonal routing deferred for class/ER).
pub fn render_graph_model_with_compartments(
    model: &crate::render::scene::GraphModel,
    layout: &crate::layout::FlowchartLayout,
    options: RenderOptions,
) -> Result<String, CanvasError> {
    use crate::render::text::canvas_to_string_trimmed;
    use std::collections::BTreeMap;

    let col_gap = 3usize;
    let row_gap = 1usize;

    // Layer widths and per-node metrics.
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

    // Cursor x per layer.
    let mut layer_x0 = Vec::<usize>::with_capacity(layout.layers().len());
    let mut x = 0usize;
    for (i, inner) in layer_inner_widths.iter().enumerate() {
        layer_x0.push(x);
        x = x.saturating_add(inner.saturating_add(2));
        if i + 1 < layer_inner_widths.len() {
            x = x.saturating_add(col_gap);
        }
    }
    let width = x.max(1);

    // Place nodes: y stack within each layer independently; take global max height.
    struct Placed {
        x0: usize,
        y0: usize,
        x1: usize,
        mid_y: usize,
        mid_x_left: usize,
        mid_x_right: usize,
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
            placed.insert(
                node_id.clone(),
                Placed { x0, y0: y, x1, mid_y, mid_x_left: x0, mid_x_right: x1 },
            );
            height = height.max(y1.saturating_add(1));
            y = y1.saturating_add(1).saturating_add(row_gap);
        }
    }

    let mut canvas = Canvas::new(width, height.max(1))?;
    for (node_id, node) in model.nodes() {
        let Some(p) = placed.get(node_id) else {
            continue;
        };
        let inner = p.x1.saturating_sub(p.x0).saturating_sub(1);
        paint_graph_node_box(&mut canvas, node, p.x0, p.y0, inner, options)?;
    }

    for edge in model.edges().values() {
        let Some(from) = placed.get(edge.from_node_id()) else {
            continue;
        };
        let Some(to) = placed.get(edge.to_node_id()) else {
            continue;
        };
        let y = from.mid_y;
        let y2 = to.mid_y;
        let (x_start, x_end) = if from.x1 < to.x0 {
            (from.mid_x_right, to.mid_x_left)
        } else if to.x1 < from.x0 {
            (from.mid_x_left, to.mid_x_right)
        } else {
            // same column-ish: vertical-ish stub
            continue;
        };
        let left = x_start.min(x_end);
        let right = x_start.max(x_end);
        if left + 1 < right {
            canvas.draw_hline(left.saturating_add(1), right.saturating_sub(1), y)?;
        }
        if y != y2 {
            let bend_x = (left + right) / 2;
            canvas.draw_vline(bend_x, y.min(y2), y.max(y2))?;
        }
        // Caps at ends (outward from target/source boxes).
        if let Some(ch) = edge.end_cap().glyph(if to.x0 > from.x1 { -1 } else { 1 }, 0) {
            let cap_x = if to.x0 > from.x1 { to.mid_x_left } else { to.mid_x_right };
            let _ = canvas.set(cap_x, y2, ch);
        }
        if let Some(ch) = edge.start_cap().glyph(if from.x1 < to.x0 { 1 } else { -1 }, 0) {
            let cap_x = if from.x1 < to.x0 { from.mid_x_right } else { from.mid_x_left };
            let _ = canvas.set(cap_x, y, ch);
        }
        if let Some(label) = edge.label() {
            let lx = (left + right) / 2;
            let _ = canvas.write_str(lx.saturating_sub(text_len(label) / 2), y, label);
        }
    }

    Ok(canvas_to_string_trimmed(&canvas))
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
}
