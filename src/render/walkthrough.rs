// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::fmt;

use crate::model::ids::WalkthroughNodeId;
use crate::model::walkthrough::Walkthrough;

use super::text::{canvas_to_string_trimmed, text_len, truncate_with_ellipsis};
use super::{Canvas, CanvasError};

const BOX_HEIGHT: usize = 3;
const COL_GAP: usize = 4;
const MIN_BOX_INNER_WIDTH: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NodeRender<'a> {
    node_id: &'a WalkthroughNodeId,
    title: &'a str,
    box_x0: usize,
    box_x1: usize,
    box_inner_width: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkthroughRenderError {
    Canvas(CanvasError),
    MissingNode { node_id: WalkthroughNodeId },
}

impl fmt::Display for WalkthroughRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canvas(err) => write!(f, "canvas error: {err}"),
            Self::MissingNode { node_id } => write!(f, "missing node {node_id} in walkthrough"),
        }
    }
}

impl std::error::Error for WalkthroughRenderError {}

impl From<CanvasError> for WalkthroughRenderError {
    fn from(value: CanvasError) -> Self {
        Self::Canvas(value)
    }
}

/// Deterministic baseline Unicode renderer for walkthroughs.
///
/// Baseline limitations:
/// - Nodes are rendered in `walkthrough.nodes()` order (left-to-right).
/// - Only edges between adjacent nodes in that order are rendered.
pub fn render_walkthrough_unicode(
    walkthrough: &Walkthrough,
) -> Result<String, WalkthroughRenderError> {
    let nodes = walkthrough.nodes();
    if nodes.is_empty() {
        return Ok(String::new());
    }

    let mut node_renders = Vec::<NodeRender>::with_capacity(nodes.len());
    let mut cursor_x = 0usize;

    for node in nodes {
        let (box_inner_width, box_total_width) = box_widths(node.title());
        let box_x0 = cursor_x;
        let box_x1 = box_x0 + box_total_width - 1;

        node_renders.push(NodeRender {
            node_id: node.node_id(),
            title: node.title(),
            box_x0,
            box_x1,
            box_inner_width,
        });

        cursor_x = box_x1 + 1 + COL_GAP;
    }

    let width = node_renders.last().map(|n| n.box_x1 + 1).unwrap_or(1);
    let mut canvas = Canvas::new(width, BOX_HEIGHT)?;

    for n in &node_renders {
        canvas.draw_box(n.box_x0, 0, n.box_x1, BOX_HEIGHT - 1)?;
        let clipped = truncate_with_ellipsis(n.title, n.box_inner_width);
        let clipped_len = text_len(&clipped);
        let left_pad = (n.box_inner_width.saturating_sub(clipped_len)) / 2;
        let label_x = n.box_x0 + 1 + left_pad;
        canvas.write_str(label_x, 1, &clipped)?;
    }

    for edge in walkthrough.edges() {
        let from_idx =
            nodes.iter().position(|n| n.node_id() == edge.from_node_id()).ok_or_else(|| {
                WalkthroughRenderError::MissingNode { node_id: edge.from_node_id().clone() }
            })?;
        let to_idx =
            nodes.iter().position(|n| n.node_id() == edge.to_node_id()).ok_or_else(|| {
                WalkthroughRenderError::MissingNode { node_id: edge.to_node_id().clone() }
            })?;

        if from_idx + 1 == to_idx {
            draw_arrow_right(&mut canvas, node_renders[from_idx], node_renders[to_idx])?;
        } else if to_idx + 1 == from_idx {
            draw_arrow_left(&mut canvas, node_renders[from_idx], node_renders[to_idx])?;
        }
    }

    Ok(canvas_to_string_trimmed(&canvas))
}

fn draw_arrow_right(
    canvas: &mut Canvas,
    from: NodeRender<'_>,
    to: NodeRender<'_>,
) -> Result<(), WalkthroughRenderError> {
    let y = 1usize;
    let start_x = from.box_x1 + 1;
    let arrow_head_x = to.box_x0.saturating_sub(1);
    if arrow_head_x < start_x || y >= canvas.height() {
        return Ok(());
    }

    canvas.draw_hline(start_x, arrow_head_x, y)?;
    canvas.set(arrow_head_x, y, '▶')?;
    Ok(())
}

fn draw_arrow_left(
    canvas: &mut Canvas,
    from: NodeRender<'_>,
    to: NodeRender<'_>,
) -> Result<(), WalkthroughRenderError> {
    let y = 1usize;
    let start_x = from.box_x0.saturating_sub(1);
    let arrow_head_x = to.box_x1 + 1;
    if start_x < arrow_head_x || y >= canvas.height() {
        return Ok(());
    }

    canvas.draw_hline(arrow_head_x, start_x, y)?;
    canvas.set(arrow_head_x, y, '◀')?;
    Ok(())
}

fn box_widths(title: &str) -> (usize, usize) {
    let title_len = text_len(title);
    let mut inner_width = (title_len + 2).max(MIN_BOX_INNER_WIDTH);
    let mut total_width = inner_width + 2;

    // Keep widths odd so arrows naturally land on a single center-ish cell.
    if total_width % 2 == 0 {
        total_width += 1;
        inner_width += 1;
    }

    (inner_width, total_width)
}

#[cfg(test)]
mod tests {
    use super::render_walkthrough_unicode;
    use crate::model::ids::{WalkthroughId, WalkthroughNodeId};
    use crate::model::walkthrough::{Walkthrough, WalkthroughEdge, WalkthroughNode};

    #[test]
    fn snapshot_two_nodes_one_edge() {
        let mut wt =
            Walkthrough::new(WalkthroughId::new("wt:demo").expect("walkthrough id"), "Demo");

        let n_start = WalkthroughNodeId::new("wtn:start").expect("node id");
        let n_end = WalkthroughNodeId::new("wtn:end").expect("node id");

        wt.nodes_mut().push(WalkthroughNode::new(n_start.clone(), "Start"));
        wt.nodes_mut().push(WalkthroughNode::new(n_end.clone(), "End"));
        wt.edges_mut().push(WalkthroughEdge::new(n_start, n_end, "next"));

        let rendered = render_walkthrough_unicode(&wt).expect("render");
        assert_eq!(rendered, "┌───────┐    ┌─────┐\n│ Start │───▶│ End │\n└───────┘    └─────┘");
    }
}
