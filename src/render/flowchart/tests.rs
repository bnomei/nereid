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
use crate::layout::layout_flowchart;
use crate::model::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
use crate::model::ids::ObjectId;
use crate::model::{DiagramId, ObjectRef};
use crate::render::RenderOptions;
use std::collections::{BTreeMap, BTreeSet};

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

fn assert_flowchart_connectors_do_not_enter_node_interiors(ast: &FlowchartAst) {
    let layout = layout_flowchart(ast).expect("layout");
    let plan =
        super::FlowchartRenderPlan::build(ast, &layout, RenderOptions::default()).expect("plan");

    let interior_cells = node_interior_cells(&plan.node_renders);

    for (edge_idx, (edge_id, edge)) in ast.edges().iter().enumerate() {
        let from = plan
            .node_renders
            .get(edge.from_node_id())
            .copied()
            .expect("from placement");
        let to = plan
            .node_renders
            .get(edge.to_node_id())
            .copied()
            .expect("to placement");
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
            "┌───┐   ┌───┐   ┌───┐\n│ A ├──┬┤ B ├──┬┤ D │\n└───┘  │└───┘  │└───┘\n       │       │\n       │       │\n       │┌───┐  │\n       └┤ C ├──┘\n        └───┘"
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
        RenderOptions {
            show_notes: true,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 0,
        },
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
        "┌───┐   ┌───┐   ┌───┐\n│ A ├──┬┤ B ├──┬┤ D │\n└───┘  │└───┘  │└───┘\n       │       │\n       └───────┘"
    );
}

#[test]
fn annotated_render_indexes_nodes_and_edges() {
    let mut ast = FlowchartAst::default();
    let n_a = oid("n:a");
    let n_b = oid("n:b");

    ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));

    ast.edges_mut()
        .insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));

    let layout = layout_flowchart(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-flow").expect("diagram id");
    let annotated = render_flowchart_unicode_annotated(&diagram_id, &ast, &layout).expect("render");

    assert_eq!(
        annotated.text,
        render_flowchart_unicode(&ast, &layout).expect("plain render")
    );

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
    assert!(edge_text.contains('─'));
    assert!(edge_text.contains('├') || edge_text.contains('┤'));
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
        let from = plan
            .node_renders
            .get(edge.from_node_id())
            .copied()
            .expect("from placement");
        let to = plan
            .node_renders
            .get(edge.to_node_id())
            .copied()
            .expect("to placement");
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

    let e_0011: ObjectRef = "d:om-20-motifs/flow/edge/e:0011"
        .parse()
        .expect("e:0011 ref");
    let e_0009: ObjectRef = "d:om-20-motifs/flow/edge/e:0009"
        .parse()
        .expect("e:0009 ref");
    let e_0007: ObjectRef = "d:om-20-motifs/flow/edge/e:0007"
        .parse()
        .expect("e:0007 ref");

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

    let e_lions_sleep: ObjectRef = "d:om-20-motifs/flow/edge/e:0011"
        .parse()
        .expect("e:0011 ref");
    let e_endurance_fight: ObjectRef = "d:om-20-motifs/flow/edge/e:0007"
        .parse()
        .expect("e:0007 ref");
    let e_luck_streak: ObjectRef = "d:om-20-motifs/flow/edge/e:0009"
        .parse()
        .expect("e:0009 ref");

    let lions_sleep = spans_to_cells(
        annotated
            .highlight_index
            .get(&e_lions_sleep)
            .expect("lions->sleep spans"),
    );
    let endurance_fight = spans_to_cells(
        annotated
            .highlight_index
            .get(&e_endurance_fight)
            .expect("endurance->fight spans"),
    );
    let luck_streak = spans_to_cells(
        annotated
            .highlight_index
            .get(&e_luck_streak)
            .expect("luck->streak spans"),
    );

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
        annotated
            .highlight_index
            .get(&e_salao_resolve)
            .expect("salao->resolve spans"),
    );
    let parents_separation = spans_to_cells(
        annotated
            .highlight_index
            .get(&e_parents_separation)
            .expect("parents->separation spans"),
    );

    assert_no_overlap_or_side_touch(
        &salao_resolve,
        &parents_separation,
        "om-05-luck e:0010 vs e:0003",
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

    let mast_skiff = spans_to_cells(
        annotated
            .highlight_index
            .get(&e_mast_skiff)
            .expect("mast->skiff spans"),
    );
    let tools_club = spans_to_cells(
        annotated
            .highlight_index
            .get(&e_tools_club)
            .expect("tools->club spans"),
    );

    assert_no_overlap_or_side_touch(&mast_skiff, &tools_club, "om-02-gear e:0007 vs e:0006");
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

    let from_0007 = plan
        .node_renders
        .get(edge_0007.from_node_id())
        .copied()
        .expect("from 0007 placement");
    let to_0007 = plan
        .node_renders
        .get(edge_0007.to_node_id())
        .copied()
        .expect("to 0007 placement");
    let from_0010 = plan
        .node_renders
        .get(edge_0010.from_node_id())
        .copied()
        .expect("from 0010 placement");
    let to_0010 = plan
        .node_renders
        .get(edge_0010.to_node_id())
        .copied()
        .expect("to 0010 placement");

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
    assert_eq!(
        gap_0007, gap_0010,
        "expected edges to share the same start gap\n{rendered}"
    );

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
