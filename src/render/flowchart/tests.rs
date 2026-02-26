// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::super::test_utils::collect_spanned_text;
use super::{
    render_flowchart_unicode, render_flowchart_unicode_annotated,
    render_flowchart_unicode_annotated_with_options, render_flowchart_unicode_with_options,
};
use crate::layout::flowchart::route_flowchart_edges_orthogonal_with_diagnostics;
use crate::layout::layout_flowchart;
use crate::model::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
use crate::model::ids::ObjectId;
use crate::model::{DiagramId, ObjectRef};
use crate::render::{HighlightIndex, RenderOptions};
use std::collections::{BTreeMap, BTreeSet};

const DETERMINISM_REPEAT_RUNS: usize = 100;

fn assert_highlight_spans_in_bounds(
    fixture_id: &str,
    text: &str,
    highlight_index: &HighlightIndex,
) {
    let lines = text.split('\n').collect::<Vec<_>>();

    for (object_ref, spans) in highlight_index {
        assert!(
            !spans.is_empty(),
            "flow fixture `{fixture_id}` produced empty spans for {object_ref}"
        );
        for (span_idx, (y, x0, x1)) in spans.iter().copied().enumerate() {
            let line = lines.get(y).unwrap_or_else(|| {
                panic!(
                    "flow fixture `{fixture_id}` has out-of-bounds y={y} for {object_ref} span #{span_idx}"
                )
            });
            let line_len = super::super::text::text_len(line);
            assert!(
                line_len > 0,
                "flow fixture `{fixture_id}` has span on empty line for {object_ref} span #{span_idx}: (y={y}, x0={x0}, x1={x1})"
            );
            assert!(
                x0 <= x1,
                "flow fixture `{fixture_id}` has inverted span for {object_ref} span #{span_idx}: (y={y}, x0={x0}, x1={x1})"
            );
            assert!(
                x1 < line_len,
                "flow fixture `{fixture_id}` has out-of-bounds x1 for {object_ref} span #{span_idx}: (y={y}, x0={x0}, x1={x1}, line_len={line_len})"
            );
        }
    }
}

fn oid(value: &str) -> ObjectId {
    ObjectId::new(value).expect("object id")
}

fn node_interior_cells(
    node_renders: &BTreeMap<ObjectId, super::NodeRender>,
) -> BTreeSet<(usize, usize)> {
    let mut cells = BTreeSet::<(usize, usize)>::new();
    for render in node_renders.values() {
        if render.box_x1 <= render.box_x0 + 1 || render.box_y1 <= render.box_y0 + 1 {
            continue;
        }

        for y in (render.box_y0 + 1)..render.box_y1 {
            for x in (render.box_x0 + 1)..render.box_x1 {
                cells.insert((x, y));
            }
        }
    }
    cells
}

fn node_box_cells_excluding(
    node_renders: &BTreeMap<ObjectId, super::NodeRender>,
    excluded: &BTreeSet<ObjectId>,
) -> BTreeSet<(usize, usize)> {
    let mut cells = BTreeSet::<(usize, usize)>::new();
    for (node_id, render) in node_renders {
        if excluded.contains(node_id) {
            continue;
        }
        for y in render.box_y0..=render.box_y1 {
            for x in render.box_x0..=render.box_x1 {
                cells.insert((x, y));
            }
        }
    }
    cells
}

fn node_label_row_cells(
    node_renders: &BTreeMap<ObjectId, super::NodeRender>,
) -> BTreeMap<ObjectId, BTreeSet<(usize, usize)>> {
    let mut by_node = BTreeMap::<ObjectId, BTreeSet<(usize, usize)>>::new();
    for (node_id, render) in node_renders {
        if render.box_x1 <= render.box_x0 + 1 || render.box_y1 <= render.box_y0 + 1 {
            continue;
        }
        let y = render.box_y0 + 1;
        let mut cells = BTreeSet::<(usize, usize)>::new();
        for x in (render.box_x0 + 1)..render.box_x1 {
            cells.insert((x, y));
        }
        by_node.insert(node_id.clone(), cells);
    }
    by_node
}

fn assert_spans_avoid_cells(
    edge_id: &ObjectId,
    spans: &[super::LineSpan],
    forbidden: &BTreeSet<(usize, usize)>,
) {
    for (y, x0, x1) in spans.iter().copied() {
        let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for x in min_x..=max_x {
            if forbidden.contains(&(x, y)) {
                panic!("edge {edge_id} enters node interior at (x={x}, y={y})");
            }
        }
    }
}

fn spans_to_cells(spans: &[super::super::LineSpan]) -> BTreeSet<(usize, usize)> {
    let mut out = BTreeSet::new();
    for (y, x0, x1) in spans.iter().copied() {
        let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for x in min_x..=max_x {
            out.insert((x, y));
        }
    }
    out
}

fn is_connector_glyph(ch: char) -> bool {
    matches!(ch, '─' | '│' | '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼')
}

fn arrow_tail_delta(ch: char) -> Option<(isize, isize)> {
    match ch {
        '▶' => Some((-1, 0)),
        '◀' => Some((1, 0)),
        '▲' => Some((0, 1)),
        '▼' => Some((0, -1)),
        _ => None,
    }
}

fn assert_no_overlap_or_side_touch(
    a_cells: &BTreeSet<(usize, usize)>,
    b_cells: &BTreeSet<(usize, usize)>,
    pair_label: &str,
) {
    for &(x, y) in a_cells {
        if b_cells.contains(&(x, y)) {
            panic!("expected {pair_label} to be disjoint, but both cover ({x}, {y})");
        }

        if let Some(nx) = x.checked_sub(1) {
            if b_cells.contains(&(nx, y)) {
                panic!(
                        "expected {pair_label} to keep horizontal clearance, but cells touch at ({x}, {y}) and ({nx}, {y})"
                    );
            }
        }
        if let Some(nx) = x.checked_add(1) {
            if b_cells.contains(&(nx, y)) {
                panic!(
                        "expected {pair_label} to keep horizontal clearance, but cells touch at ({x}, {y}) and ({nx}, {y})"
                    );
            }
        }
        if let Some(ny) = y.checked_sub(1) {
            if b_cells.contains(&(x, ny)) {
                panic!(
                        "expected {pair_label} to keep vertical clearance, but cells touch at ({x}, {y}) and ({x}, {ny})"
                    );
            }
        }
        if let Some(ny) = y.checked_add(1) {
            if b_cells.contains(&(x, ny)) {
                panic!(
                        "expected {pair_label} to keep vertical clearance, but cells touch at ({x}, {y}) and ({x}, {ny})"
                    );
            }
        }
    }
}

fn route_gap_set(
    route: &[crate::layout::GridPoint],
    from_layer: usize,
    to_layer: usize,
    layer_count: usize,
) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for idx in 0..route.len() {
        if let Some(gap_idx) =
            super::route_grid_x_to_lane_gap(route, idx, from_layer, to_layer, layer_count)
        {
            out.insert(gap_idx);
        }
    }
    out
}

fn assert_flowchart_connectors_do_not_enter_node_interiors_with_options(
    ast: &FlowchartAst,
    options: RenderOptions,
) {
    let layout = layout_flowchart(ast).expect("layout");
    let plan = super::FlowchartRenderPlan::build(ast, &layout, options).expect("plan");

    let interior_cells = node_interior_cells(&plan.node_renders);

    for (edge_idx, (edge_id, edge)) in ast.edges().iter().enumerate() {
        let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
        let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
        let route = plan.routes.get(edge_idx).expect("route");

        for (idx, _p) in route.iter().enumerate() {
            super::route_grid_x_to_lane_x(
                route,
                idx,
                from,
                to,
                &plan.layer_metrics,
                &plan.gap_widths,
                edge_idx,
                &plan.edge_gap_lanes,
            )
            .unwrap_or_else(|| {
                panic!("edge {edge_id} route point {idx} cannot be projected to lane space")
            });
        }

        let spans = super::routed_connector_spans(
            from,
            to,
            &plan.layer_metrics,
            &plan.gap_widths,
            route,
            plan.box_height,
            edge_idx,
            &plan.edge_gap_lanes,
        );
        assert_spans_avoid_cells(edge_id, &spans, &interior_cells);
    }
}

fn assert_flowchart_connectors_do_not_enter_node_interiors(ast: &FlowchartAst) {
    assert_flowchart_connectors_do_not_enter_node_interiors_with_options(
        ast,
        RenderOptions::default(),
    );
}

#[test]
fn horizontal_segments_bridge_over_existing_vertical_segments() {
    let mut canvas = super::super::Canvas::new(5, 5).expect("canvas");
    canvas.draw_vline(2, 0, 4).expect("vline");

    super::draw_hline_bridge_vertical(&mut canvas, 0, 4, 2).expect("hline");

    assert_eq!(canvas.get(1, 2).expect("cell"), '─');
    assert_eq!(canvas.get(2, 2).expect("cell"), '│');
    assert_eq!(canvas.get(3, 2).expect("cell"), '─');
}

#[test]
fn snapshot_small_dag() {
    let ast = crate::model::fixtures::flowchart_small_dag();

    let layout = layout_flowchart(&ast).expect("layout");
    let rendered = render_flowchart_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            "┌───┐   ┌───┐   ┌───┐\n│ A ├──▶│ B ├──▶│ D │\n└───┘  │└───┘  │└───┘\n       │       │\n       │       │\n       │┌───┐  │\n       └┤ C ├──┘\n        └───┘"
        );
}

#[test]
fn snapshot_single_node_notes_toggle() {
    let mut ast = FlowchartAst::default();
    let mut node = FlowNode::new("Node");
    node.set_note(Some("note"));
    ast.nodes_mut().insert(oid("n:a"), node);

    let layout = layout_flowchart(&ast).expect("layout");

    let notes_off = render_flowchart_unicode_with_options(
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 0,
        },
    )
    .expect("render");
    assert_eq!(notes_off, "┌───────┐\n│ Node  │\n└───────┘");

    let notes_on = render_flowchart_unicode_with_options(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 0 },
    )
    .expect("render");
    assert_eq!(notes_on, "┌───────┐\n│ Node  │\n│ note  │\n└───────┘");
}

#[test]
fn snapshot_routes_around_obstacle() {
    let ast = crate::model::fixtures::flowchart_obstacle_route();

    let layout = layout_flowchart(&ast).expect("layout");
    let rendered = render_flowchart_unicode(&ast, &layout).expect("render");

    assert_eq!(
        rendered,
        "┌───┐   ┌───┐   ┌───┐\n│ A ├──▶│ B ├──▶│ D │\n└───┘  │└───┘  │└───┘\n       │       │\n       └───────┘"
    );
}

#[test]
fn renders_default_flow_arrowhead_at_edge_end() {
    use crate::format::mermaid::parse_flowchart;

    let ast = parse_flowchart("flowchart LR\nA --> B\n").expect("parse");
    let layout = layout_flowchart(&ast).expect("layout");
    let rendered = render_flowchart_unicode(&ast, &layout).expect("render");

    assert!(
        rendered.contains('▶'),
        "expected default flow edge to render an end arrowhead:\n{rendered}"
    );
}

#[test]
fn renders_flow_arrowheads_on_both_endpoints_when_connector_requests_both() {
    use crate::format::mermaid::parse_flowchart;

    let ast = parse_flowchart("flowchart LR\nA <--> B\n").expect("parse");
    let layout = layout_flowchart(&ast).expect("layout");
    let rendered = render_flowchart_unicode_with_options(
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 3,
        },
    )
    .expect("render");

    assert!(rendered.contains('◀'), "expected start arrowhead for `<-->` connector:\n{rendered}");
    assert!(rendered.contains('▶'), "expected end arrowhead for `<-->` connector:\n{rendered}");
}

#[test]
fn renders_distinct_mixed_endpoint_caps_for_multiple_incoming_edges_to_same_node() {
    use crate::format::mermaid::parse_flowchart;

    let input = r#"
        flowchart LR
        A --> T
        B ---o T
        C ---x T
    "#;
    let ast = parse_flowchart(input).expect("parse");
    let layout = layout_flowchart(&ast).expect("layout");
    let rendered = render_flowchart_unicode_with_options(
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 3,
        },
    )
    .expect("render");

    assert!(
        rendered.chars().any(|ch| matches!(ch, '▶' | '○' | '✕')),
        "expected at least one visible endpoint cap:\n{rendered}"
    );
    assert!(rendered.contains('○'), "expected circle cap for `---o` incoming edge:\n{rendered}");
    assert!(rendered.contains('✕'), "expected cross cap for `---x` incoming edge:\n{rendered}");
}

#[test]
fn demo_arc_arrow_caps_keep_visible_connector_tail_cells() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-03-arc.mmd");
    let input = std::fs::read_to_string(&root).expect("read arc fixture");
    let ast = parse_flowchart(&input).expect("parse arc fixture");
    let layout = layout_flowchart(&ast).expect("layout");
    let rendered = render_flowchart_unicode_with_options(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 0 },
    )
    .expect("render");

    let lines = rendered.lines().map(|line| line.chars().collect::<Vec<_>>()).collect::<Vec<_>>();
    let mut arrow_count = 0usize;

    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.iter().copied().enumerate() {
            let Some((dx, dy)) = arrow_tail_delta(ch) else {
                continue;
            };
            arrow_count = arrow_count.saturating_add(1);

            let tail_x = if dx < 0 {
                x.checked_sub(dx.unsigned_abs()).expect("arrow tail x in bounds")
            } else {
                x.checked_add(dx as usize).expect("arrow tail x in bounds")
            };
            let tail_y = if dy < 0 {
                y.checked_sub(dy.unsigned_abs()).expect("arrow tail y in bounds")
            } else {
                y.checked_add(dy as usize).expect("arrow tail y in bounds")
            };

            let tail = lines.get(tail_y).and_then(|row| row.get(tail_x)).copied().unwrap_or(' ');
            assert!(
                is_connector_glyph(tail),
                "expected connector glyph before arrow `{ch}` at ({x},{y}), found `{tail}`\n{rendered}"
            );
        }
    }

    assert!(arrow_count > 0, "fixture should contain arrow caps:\n{rendered}");
}

#[test]
fn demo_arc_harpoon_keeps_min_three_columns_between_adjacent_nodes_with_tui_gap() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-03-arc.mmd");
    let input = std::fs::read_to_string(&root).expect("read arc fixture");
    let ast = parse_flowchart(&input).expect("parse arc fixture");
    let layout = layout_flowchart(&ast).expect("layout");
    let plan = super::FlowchartRenderPlan::build(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("plan");

    let fight = plan.node_renders.get(&oid("n:fight")).copied().expect("n:fight");
    let harpoon = plan.node_renders.get(&oid("n:harpoon")).copied().expect("n:harpoon");
    let lash = plan.node_renders.get(&oid("n:lash")).copied().expect("n:lash");
    assert_eq!(
        fight.layer.abs_diff(harpoon.layer),
        1,
        "fixture assumption: fight->harpoon adjacent"
    );
    assert_eq!(harpoon.layer.abs_diff(lash.layer), 1, "fixture assumption: harpoon->lash adjacent");

    let left_corridor = harpoon.box_x0.saturating_sub(fight.box_x1.saturating_add(1));
    let right_corridor = lash.box_x0.saturating_sub(harpoon.box_x1.saturating_add(1));
    assert!(
        left_corridor >= 3,
        "expected >=3 columns between n:fight and n:harpoon with TUI gap, got {left_corridor}"
    );
    assert!(
        right_corridor >= 3,
        "expected >=3 columns between n:harpoon and n:lash with TUI gap, got {right_corridor}"
    );
}

#[test]
fn demo_arc_edge_0013_arrow_keeps_corner_tail_cell() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-03-arc.mmd");
    let input = std::fs::read_to_string(&root).expect("read arc fixture");
    let ast = parse_flowchart(&input).expect("parse arc fixture");
    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("om-03-arc").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated_with_options(
        &diagram_id,
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("render");
    let plan = super::FlowchartRenderPlan::build(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("plan");
    let edge_idx = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &oid("e:0013")).then_some(idx))
        .expect("e:0013 index");
    let cap = plan.edge_caps[edge_idx].end.expect("e:0013 end cap");
    let tail_x = if cap.outward_dx < 0 {
        cap.x.checked_sub(cap.outward_dx.unsigned_abs() as usize).expect("tail x")
    } else {
        cap.x.checked_add(cap.outward_dx as usize).expect("tail x")
    };
    let tail_y = if cap.outward_dy < 0 {
        cap.y.checked_sub(cap.outward_dy.unsigned_abs() as usize).expect("tail y")
    } else {
        cap.y.checked_add(cap.outward_dy as usize).expect("tail y")
    };
    let tail =
        annotated.text.lines().nth(tail_y).and_then(|line| line.chars().nth(tail_x)).unwrap_or(' ');
    assert!(
        matches!(tail, '┌' | '┐' | '└' | '┘'),
        "expected e:0013 tail before cap to be a corner, got `{tail}`:\n{}",
        annotated.text
    );
}

#[test]
fn demo_arc_edge_0014_arrow_keeps_tee_tail_cell() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-03-arc.mmd");
    let input = std::fs::read_to_string(&root).expect("read arc fixture");
    let ast = parse_flowchart(&input).expect("parse arc fixture");
    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("om-03-arc").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated_with_options(
        &diagram_id,
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("render");
    let plan = super::FlowchartRenderPlan::build(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("plan");
    let e0004_idx = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &oid("e:0004")).then_some(idx))
        .expect("e:0004 index");
    let cap = plan.edge_caps[e0004_idx].end.expect("e:0004 end cap");
    let tail_x = if cap.outward_dx < 0 {
        cap.x.checked_sub(cap.outward_dx.unsigned_abs() as usize).expect("tail x")
    } else {
        cap.x.checked_add(cap.outward_dx as usize).expect("tail x")
    };
    let tail_y = if cap.outward_dy < 0 {
        cap.y.checked_sub(cap.outward_dy.unsigned_abs() as usize).expect("tail y")
    } else {
        cap.y.checked_add(cap.outward_dy as usize).expect("tail y")
    };
    let tail =
        annotated.text.lines().nth(tail_y).and_then(|line| line.chars().nth(tail_x)).unwrap_or(' ');
    let e0014_ref: ObjectRef = "d:om-03-arc/flow/edge/e:0014".parse().expect("edge ref");
    let e0014_cells =
        spans_to_cells(annotated.highlight_index.get(&e0014_ref).expect("e:0014 spans"));
    assert!(
        e0014_cells.contains(&(tail_x, tail_y)),
        "expected tee-tail before shared cap to belong to e:0014 highlight:\n{}",
        annotated.text
    );
    assert!(
        matches!(tail, '├' | '┤' | '┬' | '┴' | '┼'),
        "expected merge before e:0014 shared cap to be a tee/cross, got `{tail}`:\n{}",
        annotated.text
    );
}

#[test]
fn annotated_render_indexes_nodes_and_edges() {
    let mut ast = FlowchartAst::default();
    let n_a = oid("n:a");
    let n_b = oid("n:b");

    ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));

    ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-flow").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated(&diagram_id, &ast, &layout).expect("render");

    assert_eq!(annotated.text, render_flowchart_unicode(&ast, &layout).expect("plain render"));

    let a_ref: ObjectRef = "d:d-flow/flow/node/n:a".parse().expect("object ref");
    let b_ref: ObjectRef = "d:d-flow/flow/node/n:b".parse().expect("object ref");
    let e_ref: ObjectRef = "d:d-flow/flow/edge/e:ab".parse().expect("object ref");

    let a_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&a_ref).expect("a spans"),
    );
    assert!(a_text.contains("A"));

    let b_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&b_ref).expect("b spans"),
    );
    assert!(b_text.contains("B"));

    let edge_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&e_ref).expect("edge spans"),
    );
    assert!(
        edge_text.chars().any(|ch| matches!(ch, '─' | '├' | '┤' | '▶' | '◀' | '○' | '✕')),
        "expected edge highlight text to contain connector or cap glyphs, got: {edge_text:?}"
    );
}

#[test]
fn annotated_edge_spans_include_endpoint_cap_cells() {
    use crate::format::mermaid::parse_flowchart;

    let ast = parse_flowchart("flowchart LR\nA --> B\n").expect("parse");
    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-flow-caps").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated(&diagram_id, &ast, &layout).expect("render");

    let mut cap_pos = None::<(usize, usize)>;
    for (y, line) in annotated.text.lines().enumerate() {
        if let Some(x) = line.chars().position(|ch| ch == '▶') {
            cap_pos = Some((x, y));
            break;
        }
    }
    let (cap_x, cap_y) = cap_pos.expect("arrowhead cell in rendered text");

    let edge_ref: ObjectRef = "d:d-flow-caps/flow/edge/e:0001".parse().expect("edge ref");
    let spans = annotated.highlight_index.get(&edge_ref).expect("edge spans");
    let cells = spans_to_cells(spans);
    assert!(
        cells.contains(&(cap_x, cap_y)),
        "expected edge highlight spans to include cap cell ({cap_x}, {cap_y})"
    );
}

#[test]
fn connectors_do_not_enter_node_box_interior_cells() {
    let small = crate::model::fixtures::flowchart_small_dag();
    assert_flowchart_connectors_do_not_enter_node_interiors(&small);

    let obstacle = crate::model::fixtures::flowchart_obstacle_route();
    assert_flowchart_connectors_do_not_enter_node_interiors(&obstacle);

    let regression = crate::model::fixtures::flowchart_node_overlap_avoidance_regression();
    assert_flowchart_connectors_do_not_enter_node_interiors(&regression);
}

#[test]
fn connectors_do_not_enter_demo_routing_fixture_node_interiors() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("demo-t-flow-routing.mmd");
    let input = std::fs::read_to_string(&root).expect("read demo routing fixture");
    let ast = parse_flowchart(&input).expect("parse demo routing fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let plan =
        super::FlowchartRenderPlan::build(&ast, &layout, RenderOptions::default()).expect("plan");
    let interior_cells = node_interior_cells(&plan.node_renders);

    for (idx, (edge_id, edge)) in ast.edges().iter().enumerate() {
        let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
        let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
        let route = plan.routes.get(idx).expect("route");

        let spans = if route.len() < 2 {
            super::connector_spans(from, to)
        } else {
            super::routed_connector_spans(
                from,
                to,
                &plan.layer_metrics,
                &plan.gap_widths,
                route,
                plan.box_height,
                idx,
                &plan.edge_gap_lanes,
            )
        };

        assert_spans_avoid_cells(edge_id, &spans, &interior_cells);
    }
}

#[test]
fn demo_motifs_selected_edges_do_not_touch() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-20-motifs.mmd");
    let input = std::fs::read_to_string(&root).expect("read motifs fixture");
    let ast = parse_flowchart(&input).expect("parse motifs fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("om-20-motifs").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated(&diagram_id, &ast, &layout).expect("render");

    let e_0011: ObjectRef = "d:om-20-motifs/flow/edge/e:0011".parse().expect("e:0011 ref");
    let e_0009: ObjectRef = "d:om-20-motifs/flow/edge/e:0009".parse().expect("e:0009 ref");
    let e_0007: ObjectRef = "d:om-20-motifs/flow/edge/e:0007".parse().expect("e:0007 ref");

    let cells_0011 = spans_to_cells(annotated.highlight_index.get(&e_0011).expect("e:0011"));
    let cells_0009 = spans_to_cells(annotated.highlight_index.get(&e_0009).expect("e:0009"));
    let cells_0007 = spans_to_cells(annotated.highlight_index.get(&e_0007).expect("e:0007"));

    for cell in &cells_0011 {
        assert!(
            !cells_0009.contains(cell),
            "expected e:0011 and e:0009 to be disjoint, but both cover {cell:?}"
        );
        assert!(
            !cells_0007.contains(cell),
            "expected e:0011 and e:0007 to be disjoint, but both cover {cell:?}"
        );
    }
}

#[test]
fn demo_motifs_lions_sleep_keeps_one_cell_clearance_with_tui_gap() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-20-motifs.mmd");
    let input = std::fs::read_to_string(&root).expect("read motifs fixture");
    let ast = parse_flowchart(&input).expect("parse motifs fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("om-20-motifs").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated_with_options(
        &diagram_id,
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 2,
        },
    )
    .expect("render");

    let e_lions_sleep: ObjectRef = "d:om-20-motifs/flow/edge/e:0011".parse().expect("e:0011 ref");
    let e_endurance_fight: ObjectRef =
        "d:om-20-motifs/flow/edge/e:0007".parse().expect("e:0007 ref");
    let e_luck_streak: ObjectRef = "d:om-20-motifs/flow/edge/e:0009".parse().expect("e:0009 ref");

    let lions_sleep =
        spans_to_cells(annotated.highlight_index.get(&e_lions_sleep).expect("lions->sleep spans"));
    let endurance_fight = spans_to_cells(
        annotated.highlight_index.get(&e_endurance_fight).expect("endurance->fight spans"),
    );
    let luck_streak =
        spans_to_cells(annotated.highlight_index.get(&e_luck_streak).expect("luck->streak spans"));

    assert_no_overlap_or_side_touch(
        &lions_sleep,
        &endurance_fight,
        "om-20-motifs e:0011 vs e:0007",
    );
    assert_no_overlap_or_side_touch(&lions_sleep, &luck_streak, "om-20-motifs e:0011 vs e:0009");
}

#[test]
fn demo_luck_salao_resolve_keeps_clearance_from_parents_separation_with_tui_gap() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-05-luck.mmd");
    let input = std::fs::read_to_string(&root).expect("read luck fixture");
    let ast = parse_flowchart(&input).expect("parse luck fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("om-05-luck").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated_with_options(
        &diagram_id,
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 2,
        },
    )
    .expect("render");

    let e_salao_resolve: ObjectRef = "d:om-05-luck/flow/edge/e:0010".parse().expect("e:0010 ref");
    let e_parents_separation: ObjectRef =
        "d:om-05-luck/flow/edge/e:0003".parse().expect("e:0003 ref");

    let salao_resolve = spans_to_cells(
        annotated.highlight_index.get(&e_salao_resolve).expect("salao->resolve spans"),
    );
    let parents_separation = spans_to_cells(
        annotated.highlight_index.get(&e_parents_separation).expect("parents->separation spans"),
    );

    assert_no_overlap_or_side_touch(
        &salao_resolve,
        &parents_separation,
        "om-05-luck e:0010 vs e:0003",
    );
}

#[test]
fn demo_routing_selected_edges_keep_clearance_with_tui_gap() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("demo-t-flow-routing.mmd");
    let input = std::fs::read_to_string(&root).expect("read demo routing fixture");
    let ast = parse_flowchart(&input).expect("parse demo routing fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("demo-t-flow-routing").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated_with_options(
        &diagram_id,
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 2,
        },
    )
    .expect("render");

    let e_0010: ObjectRef = "d:demo-t-flow-routing/flow/edge/e:0010".parse().expect("e:0010 ref");
    let e_0012: ObjectRef = "d:demo-t-flow-routing/flow/edge/e:0012".parse().expect("e:0012 ref");
    let e_0014: ObjectRef = "d:demo-t-flow-routing/flow/edge/e:0014".parse().expect("e:0014 ref");

    let cells_0010 = spans_to_cells(annotated.highlight_index.get(&e_0010).expect("e:0010"));
    let cells_0012 = spans_to_cells(annotated.highlight_index.get(&e_0012).expect("e:0012"));
    let cells_0014 = spans_to_cells(annotated.highlight_index.get(&e_0014).expect("e:0014"));

    for cell in &cells_0010 {
        assert!(
            !cells_0012.contains(cell),
            "expected demo routing e:0010 vs e:0012 to be disjoint, but both cover {cell:?}"
        );
        assert!(
            !cells_0014.contains(cell),
            "expected demo routing e:0010 vs e:0014 to be disjoint, but both cover {cell:?}"
        );
    }
}

#[test]
fn demo_routing_edge_0015_does_not_touch_metrics_node_box_with_tui_gap() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("demo-t-flow-routing.mmd");
    let input = std::fs::read_to_string(&root).expect("read demo routing fixture");
    let ast = parse_flowchart(&input).expect("parse demo routing fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("demo-t-flow-routing").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated_with_options(
        &diagram_id,
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("render");

    let e_0015: ObjectRef = "d:demo-t-flow-routing/flow/edge/e:0015".parse().expect("e:0015 ref");
    let n_metrics: ObjectRef =
        "d:demo-t-flow-routing/flow/node/n:metrics".parse().expect("n:metrics ref");

    let edge_cells = spans_to_cells(annotated.highlight_index.get(&e_0015).expect("e:0015"));
    let node_cells = spans_to_cells(annotated.highlight_index.get(&n_metrics).expect("n:metrics"));
    assert_no_overlap_or_side_touch(&edge_cells, &node_cells, "demo routing e:0015 vs n:metrics");
}

#[test]
fn demo_luck_edge_0009_avoids_node_interiors_with_tui_gap_and_notes() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-05-luck.mmd");
    let input = std::fs::read_to_string(&root).expect("read luck fixture");
    let ast = parse_flowchart(&input).expect("parse luck fixture");
    let layout = layout_flowchart(&ast).expect("layout");

    let options =
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 };
    let plan = super::FlowchartRenderPlan::build(&ast, &layout, options).expect("plan");
    let interior_cells = node_interior_cells(&plan.node_renders);

    let edge_id = oid("e:0009");
    let edge_idx = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &edge_id).then_some(idx))
        .expect("e:0009 index");
    let edge = ast.edges().get(&edge_id).expect("e:0009 edge");

    let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
    let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
    let route = plan.routes.get(edge_idx).expect("route");

    let spans = if route.len() < 2 {
        super::connector_spans(from, to)
    } else {
        super::routed_connector_spans(
            from,
            to,
            &plan.layer_metrics,
            &plan.gap_widths,
            route,
            plan.box_height,
            edge_idx,
            &plan.edge_gap_lanes,
        )
    };
    assert_spans_avoid_cells(&edge_id, &spans, &interior_cells);
}

#[test]
fn demo_luck_edge_0009_avoids_unrelated_node_boxes_with_tui_gap_and_notes() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-05-luck.mmd");
    let input = std::fs::read_to_string(&root).expect("read luck fixture");
    let ast = parse_flowchart(&input).expect("parse luck fixture");
    let layout = layout_flowchart(&ast).expect("layout");

    let options =
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 };
    let plan = super::FlowchartRenderPlan::build(&ast, &layout, options).expect("plan");

    let edge_id = oid("e:0009");
    let edge = ast.edges().get(&edge_id).expect("e:0009 edge");
    let edge_idx = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &edge_id).then_some(idx))
        .expect("e:0009 index");
    let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
    let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
    let route = plan.routes.get(edge_idx).expect("route");

    let spans = if route.len() < 2 {
        super::connector_spans(from, to)
    } else {
        super::routed_connector_spans(
            from,
            to,
            &plan.layer_metrics,
            &plan.gap_widths,
            route,
            plan.box_height,
            edge_idx,
            &plan.edge_gap_lanes,
        )
    };

    let excluded = [edge.from_node_id().clone(), edge.to_node_id().clone()]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let forbidden = node_box_cells_excluding(&plan.node_renders, &excluded);
    assert_spans_avoid_cells(&edge_id, &spans, &forbidden);
}

#[test]
fn demo_shark_types_edges_avoid_unrelated_node_boxes_with_tui_gap_and_notes() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-07-shark-types.mmd");
    let input = std::fs::read_to_string(&root).expect("read shark types fixture");
    let ast = parse_flowchart(&input).expect("parse shark types fixture");
    let layout = layout_flowchart(&ast).expect("layout");

    let options =
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 };
    let plan = super::FlowchartRenderPlan::build(&ast, &layout, options).expect("plan");

    for (edge_idx, (edge_id, edge)) in ast.edges().iter().enumerate() {
        let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
        let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
        let route = plan.routes.get(edge_idx).expect("route");

        let spans = if route.len() < 2 {
            super::connector_spans(from, to)
        } else {
            super::routed_connector_spans(
                from,
                to,
                &plan.layer_metrics,
                &plan.gap_widths,
                route,
                plan.box_height,
                edge_idx,
                &plan.edge_gap_lanes,
            )
        };

        let excluded = [edge.from_node_id().clone(), edge.to_node_id().clone()]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let forbidden = node_box_cells_excluding(&plan.node_renders, &excluded);
        assert_spans_avoid_cells(edge_id, &spans, &forbidden);
    }
}

#[test]
fn demo_shark_types_edges_do_not_cross_any_node_label_rows_with_tui_gap_and_notes() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-07-shark-types.mmd");
    let input = std::fs::read_to_string(&root).expect("read shark types fixture");
    let ast = parse_flowchart(&input).expect("parse shark types fixture");
    let layout = layout_flowchart(&ast).expect("layout");

    let options =
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 };
    let plan = super::FlowchartRenderPlan::build(&ast, &layout, options).expect("plan");
    let label_cells_by_node = node_label_row_cells(&plan.node_renders);

    for (edge_idx, (edge_id, edge)) in ast.edges().iter().enumerate() {
        let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
        let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
        let route = plan.routes.get(edge_idx).expect("route");

        let spans = if route.len() < 2 {
            super::connector_spans(from, to)
        } else {
            super::routed_connector_spans(
                from,
                to,
                &plan.layer_metrics,
                &plan.gap_widths,
                route,
                plan.box_height,
                edge_idx,
                &plan.edge_gap_lanes,
            )
        };
        let edge_cells = spans_to_cells(&spans);

        for (node_id, label_cells) in &label_cells_by_node {
            if edge_cells.iter().any(|cell| label_cells.contains(cell)) {
                panic!("edge {edge_id} crosses label row of node {node_id}");
            }
        }
    }
}

#[test]
fn demo_shark_types_connectors_do_not_enter_any_node_interiors_with_tui_gap_and_notes() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-07-shark-types.mmd");
    let input = std::fs::read_to_string(&root).expect("read shark types fixture");
    let ast = parse_flowchart(&input).expect("parse shark types fixture");

    assert_flowchart_connectors_do_not_enter_node_interiors_with_options(
        &ast,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    );
}

#[test]
fn demo_gear_mast_skiff_keeps_clearance_from_tools_club_with_tui_gap() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-02-gear.mmd");
    let input = std::fs::read_to_string(&root).expect("read gear fixture");
    let ast = parse_flowchart(&input).expect("parse gear fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("om-02-gear").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated_with_options(
        &diagram_id,
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 2,
        },
    )
    .expect("render");

    let e_mast_skiff: ObjectRef = "d:om-02-gear/flow/edge/e:0007".parse().expect("e:0007 ref");
    let e_tools_club: ObjectRef = "d:om-02-gear/flow/edge/e:0006".parse().expect("e:0006 ref");

    let mast_skiff =
        spans_to_cells(annotated.highlight_index.get(&e_mast_skiff).expect("mast->skiff spans"));
    let tools_club =
        spans_to_cells(annotated.highlight_index.get(&e_tools_club).expect("tools->club spans"));

    assert_no_overlap_or_side_touch(&mast_skiff, &tools_club, "om-02-gear e:0007 vs e:0006");
}

#[test]
fn demo_gear_edge_0014_vertical_lane_keeps_one_column_clearance_from_gap_boundaries() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-02-gear.mmd");
    let input = std::fs::read_to_string(&root).expect("read gear fixture");
    let ast = parse_flowchart(&input).expect("parse gear fixture");
    let layout = layout_flowchart(&ast).expect("layout");
    let plan = super::FlowchartRenderPlan::build(
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 2,
        },
    )
    .expect("plan");

    let edge_id = oid("e:0014");
    let edge_idx = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &edge_id).then_some(idx))
        .expect("e:0014 index");
    let edge = ast.edges().get(&edge_id).expect("e:0014 edge");
    let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
    let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
    assert_eq!(from.layer.abs_diff(to.layer), 1, "fixture assumption: adjacent layers");

    let route = plan.routes.get(edge_idx).expect("route");
    let target_gap = from.layer.min(to.layer);
    let left_boundary = plan.layer_metrics[target_gap].x1;
    let right_boundary = plan.layer_metrics[target_gap + 1].x0;

    let mut checked_segment = false;
    for seg_idx in 0..route.len().saturating_sub(1) {
        let a = route[seg_idx];
        let b = route[seg_idx + 1];
        if a.x() != b.x() || a.y() == b.y() {
            continue;
        }

        let Some(gap_idx) = super::route_grid_x_to_lane_gap(
            route,
            seg_idx,
            from.layer,
            to.layer,
            plan.layer_metrics.len(),
        ) else {
            continue;
        };
        if gap_idx != target_gap {
            continue;
        }

        let x = super::route_grid_x_to_lane_x(
            route,
            seg_idx,
            from,
            to,
            &plan.layer_metrics,
            &plan.gap_widths,
            edge_idx,
            &plan.edge_gap_lanes,
        )
        .expect("lane x for e:0014 segment");

        checked_segment = true;
        assert!(
            x >= left_boundary.saturating_add(2),
            "expected e:0014 lane to keep >=1 free column from left boundary in gap {target_gap}, got x={x}, left_boundary={left_boundary}"
        );
        assert!(
            x <= right_boundary.saturating_sub(2),
            "expected e:0014 lane to keep >=1 free column from right boundary in gap {target_gap}, got x={x}, right_boundary={right_boundary}"
        );
    }

    assert!(checked_segment, "expected e:0014 to have a vertical segment in its inter-layer gap");
}

#[test]
fn demo_om01_cast_edges_0007_and_0010_do_not_bridge_cross() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-01-cast.mmd");
    let input = std::fs::read_to_string(&root).expect("read cast fixture");
    let ast = parse_flowchart(&input).expect("parse cast fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let rendered = render_flowchart_unicode(&ast, &layout).expect("render");
    let plan =
        super::FlowchartRenderPlan::build(&ast, &layout, RenderOptions::default()).expect("plan");

    let edge_id_0007 = oid("e:0007");
    let edge_id_0010 = oid("e:0010");
    let edge_idx_0007 = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &edge_id_0007).then_some(idx))
        .expect("e:0007 index");
    let edge_idx_0010 = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &edge_id_0010).then_some(idx))
        .expect("e:0010 index");

    let edge_0007 = ast.edges().get(&edge_id_0007).expect("e:0007 edge");
    let edge_0010 = ast.edges().get(&edge_id_0010).expect("e:0010 edge");

    let from_0007 =
        plan.node_renders.get(edge_0007.from_node_id()).copied().expect("from 0007 placement");
    let to_0007 =
        plan.node_renders.get(edge_0007.to_node_id()).copied().expect("to 0007 placement");
    let from_0010 =
        plan.node_renders.get(edge_0010.from_node_id()).copied().expect("from 0010 placement");
    let to_0010 =
        plan.node_renders.get(edge_0010.to_node_id()).copied().expect("to 0010 placement");

    let route_0007 = plan.routes.get(edge_idx_0007).expect("route 0007");
    let route_0010 = plan.routes.get(edge_idx_0010).expect("route 0010");

    let gap_0007 = super::route_grid_x_to_lane_gap(
        route_0007,
        0,
        from_0007.layer,
        to_0007.layer,
        plan.layer_metrics.len(),
    )
    .expect("gap 0007");
    let gap_0010 = super::route_grid_x_to_lane_gap(
        route_0010,
        0,
        from_0010.layer,
        to_0010.layer,
        plan.layer_metrics.len(),
    )
    .expect("gap 0010");
    assert_eq!(gap_0007, gap_0010, "expected edges to share the same start gap\n{rendered}");

    let x_0007 = super::lane_x_for_gap(
        gap_0007,
        edge_idx_0007,
        &plan.layer_metrics,
        &plan.edge_gap_lanes,
        &plan.gap_widths,
    )
    .expect("lane x 0007");
    let x_0010 = super::lane_x_for_gap(
        gap_0010,
        edge_idx_0010,
        &plan.layer_metrics,
        &plan.edge_gap_lanes,
        &plan.gap_widths,
    )
    .expect("lane x 0010");

    let y_0007 = from_0007.mid_y();
    let y_0010 = from_0010.mid_y();

    if y_0007 < y_0010 {
        assert!(
                x_0007 > x_0010,
                "expected e:0007 (upper) lane x to be right of e:0010 (lower) to avoid stub bridging\n{rendered}"
            );
    } else if y_0007 > y_0010 {
        assert!(
                x_0007 < x_0010,
                "expected e:0007 (lower) lane x to be left of e:0010 (upper) to avoid stub bridging\n{rendered}"
            );
    } else {
        assert_ne!(
                x_0007, x_0010,
                "expected e:0007 and e:0010 to use different lanes when starting on the same row\n{rendered}"
            );
    }
}

#[test]
fn demo_om01_cast_tui_gap_preserves_terrace_label_and_avoids_node_interior_crossings() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-01-cast.mmd");
    let input = std::fs::read_to_string(&root).expect("read cast fixture");
    let ast = parse_flowchart(&input).expect("parse cast fixture");

    let layout = layout_flowchart(&ast).expect("layout");
    let options =
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 };

    assert_flowchart_connectors_do_not_enter_node_interiors_with_options(&ast, options);

    let rendered = render_flowchart_unicode_with_options(&ast, &layout, options).expect("render");
    assert!(
        rendered.lines().any(|line| line.contains("Terrace")),
        "expected Terrace label to stay visible with TUI gap rendering\n{rendered}"
    );
}

#[test]
fn demo_om01_cast_edge_0010_stays_in_single_gap_between_adjacent_layers() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-01-cast.mmd");
    let input = std::fs::read_to_string(&root).expect("read cast fixture");
    let ast = parse_flowchart(&input).expect("parse cast fixture");
    let layout = layout_flowchart(&ast).expect("layout");
    let plan = super::FlowchartRenderPlan::build(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("plan");

    let edge_id = oid("e:0010");
    let edge_idx = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &edge_id).then_some(idx))
        .expect("e:0010 index");
    let edge = ast.edges().get(&edge_id).expect("e:0010 edge");
    let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
    let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
    assert_eq!(from.layer.abs_diff(to.layer), 1, "fixture assumption: adjacent layers");

    let route = plan.routes.get(edge_idx).expect("route");
    let gap_set = route_gap_set(route, from.layer, to.layer, plan.layer_metrics.len());
    assert_eq!(
        gap_set.len(),
        1,
        "expected e:0010 to stay in one lane gap between adjacent layers, got {gap_set:?}"
    );
}

#[test]
fn demo_om02_gear_edge_0007_stays_in_single_gap_between_adjacent_layers() {
    use crate::format::mermaid::parse_flowchart;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("demo-session")
        .join("diagrams")
        .join("om-02-gear.mmd");
    let input = std::fs::read_to_string(&root).expect("read gear fixture");
    let ast = parse_flowchart(&input).expect("parse gear fixture");
    let layout = layout_flowchart(&ast).expect("layout");
    let plan = super::FlowchartRenderPlan::build(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 2 },
    )
    .expect("plan");

    let edge_id = oid("e:0007");
    let edge_idx = ast
        .edges()
        .keys()
        .enumerate()
        .find_map(|(idx, id)| (id == &edge_id).then_some(idx))
        .expect("e:0007 index");
    let edge = ast.edges().get(&edge_id).expect("e:0007 edge");
    let from = plan.node_renders.get(edge.from_node_id()).copied().expect("from placement");
    let to = plan.node_renders.get(edge.to_node_id()).copied().expect("to placement");
    assert_eq!(from.layer.abs_diff(to.layer), 1, "fixture assumption: adjacent layers");

    let route = plan.routes.get(edge_idx).expect("route");
    let gap_set = route_gap_set(route, from.layer, to.layer, plan.layer_metrics.len());
    assert_eq!(
        gap_set.len(),
        1,
        "expected e:0007 to stay in one lane gap between adjacent layers, got {gap_set:?}"
    );
}

#[test]
fn repeat_run_render_flowchart_unicode_is_deterministic_for_overlap_regression_fixture() {
    let fixture_id = "flowchart-node-overlap-avoidance-regression";
    let ast = crate::model::fixtures::flowchart_node_overlap_avoidance_regression();
    let layout = layout_flowchart(&ast).expect("layout");
    let baseline = render_flowchart_unicode(&ast, &layout).expect("baseline render");

    for run_idx in 1..=DETERMINISM_REPEAT_RUNS {
        let rendered = render_flowchart_unicode(&ast, &layout).expect("repeat render");
        assert_eq!(
            rendered, baseline,
            "flowchart determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
    }
}

#[test]
fn repeat_run_render_flowchart_unicode_annotated_is_deterministic_for_overlap_regression_fixture() {
    let fixture_id = "flowchart-node-overlap-avoidance-regression";
    let ast = crate::model::fixtures::flowchart_node_overlap_avoidance_regression();
    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-flow-det").expect("diagram id");
    let baseline =
        render_flowchart_unicode_annotated(&diagram_id, &ast, &layout).expect("baseline render");

    assert_highlight_spans_in_bounds(fixture_id, &baseline.text, &baseline.highlight_index);

    for run_idx in 1..=DETERMINISM_REPEAT_RUNS {
        let annotated =
            render_flowchart_unicode_annotated(&diagram_id, &ast, &layout).expect("repeat render");
        assert_eq!(
            annotated.text, baseline.text,
            "flowchart annotated text determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
        assert_eq!(
            annotated.highlight_index, baseline.highlight_index,
            "flowchart annotated highlight determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
        assert_highlight_spans_in_bounds(fixture_id, &annotated.text, &annotated.highlight_index);
    }
}

#[test]
fn overlap_regression_fixture_reports_deterministic_routing_quality_diagnostics() {
    let fixture_id = "flowchart-node-overlap-avoidance-regression";
    let ast = crate::model::fixtures::flowchart_node_overlap_avoidance_regression();
    let layout = layout_flowchart(&ast).expect("layout");

    let (baseline_routes, baseline_diagnostics) =
        route_flowchart_edges_orthogonal_with_diagnostics(&ast, &layout);
    for run_idx in 1..=DETERMINISM_REPEAT_RUNS {
        let (next_routes, next_diagnostics) =
            route_flowchart_edges_orthogonal_with_diagnostics(&ast, &layout);
        assert_eq!(
            next_routes, baseline_routes,
            "flowchart routing determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
        assert_eq!(
            next_diagnostics, baseline_diagnostics,
            "flowchart routing diagnostics determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
    }

    assert_eq!(baseline_diagnostics.fallback_route_count, 0);
    assert_eq!(baseline_diagnostics.overlap_proxy_count, 0);
    assert_eq!(baseline_diagnostics.min_clearance_violation_count, 0);
}

#[test]
fn demo_routing_fixture_reports_expected_routing_quality_diagnostics() {
    use crate::format::mermaid::parse_flowchart;

    let relative_path = "data/demo-session/diagrams/demo-t-flow-routing.mmd";
    let full_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let input = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|err| panic!("failed reading fixture {relative_path}: {err}"));
    let ast = parse_flowchart(&input)
        .unwrap_or_else(|err| panic!("failed parsing fixture {relative_path}: {err}"));
    let layout = layout_flowchart(&ast).expect("layout");

    let (baseline_routes, baseline_diagnostics) =
        route_flowchart_edges_orthogonal_with_diagnostics(&ast, &layout);
    for run_idx in 1..=32 {
        let (next_routes, next_diagnostics) =
            route_flowchart_edges_orthogonal_with_diagnostics(&ast, &layout);
        assert_eq!(
            next_routes, baseline_routes,
            "flowchart routing determinism mismatch for fixture `{relative_path}` at run {run_idx}"
        );
        assert_eq!(
            next_diagnostics, baseline_diagnostics,
            "flowchart routing diagnostics determinism mismatch for fixture `{relative_path}` at run {run_idx}"
        );
    }

    assert_eq!(baseline_diagnostics.fallback_route_count, 7);
    assert_eq!(baseline_diagnostics.overlap_proxy_count, 5);
    assert_eq!(baseline_diagnostics.min_clearance_violation_count, 12);
}
