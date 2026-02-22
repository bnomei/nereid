// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

use nereid::model::{DiagramAst, DiagramId, Session};
use nereid::ops::{apply_ops, FlowNodePatch, FlowOp, Op};
use nereid::store::SessionFolder;

mod fixtures;
mod profiler;

use fixtures::TempDir;

struct TouchSpec {
    diagram_id: DiagramId,
    ops: Vec<Op>,
}

fn touch_first_flow_node_spec(session: &Session, new_label: &str) -> TouchSpec {
    let diagram_id = DiagramId::new("flow_000").expect("diagram id");
    let diagram = session.diagrams().get(&diagram_id).expect("session has flow_000");
    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("flow_000 should be a flowchart");
    };
    let node_id = ast.nodes().keys().next().expect("flowchart has >= 1 node").clone();

    TouchSpec {
        diagram_id,
        ops: vec![Op::Flow(FlowOp::UpdateNode {
            node_id,
            patch: FlowNodePatch { label: Some(new_label.to_owned()), shape: None },
        })],
    }
}

struct PersistEditInput {
    session: Session,
    tmp: TempDir,
    flip: bool,
}

fn checksum_persist_edit(input: &mut PersistEditInput, touch: &TouchSpec) -> u64 {
    let diagram = input.session.diagrams_mut().get_mut(&touch.diagram_id).expect("diagram exists");
    let base_rev = diagram.rev();
    let apply = apply_ops(diagram, base_rev, &touch.ops).expect("apply_ops");

    let folder = SessionFolder::new(input.tmp.path());
    folder.save_session(black_box(&input.session)).expect("save_session");

    let mut acc = 0u64;
    acc = acc.wrapping_mul(131).wrapping_add(apply.new_rev);
    acc = acc.wrapping_mul(131).wrapping_add(apply.applied as u64);
    acc = acc
        .wrapping_mul(131)
        .wrapping_add(std::fs::metadata(folder.meta_path()).expect("meta_path metadata").len());
    acc = acc.wrapping_mul(131).wrapping_add(fixtures::checksum_session(&input.session));
    acc
}

// Benchmark identity (keep stable):
// - Group name in this file: `scenario.persist_edit`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `session_25_touch_1`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn benches_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario.persist_edit");

    let session_25_touch_1 = fixtures::session::fixture(fixtures::session::Case::Session25Touch1);
    let touch_25 = touch_first_flow_node_spec(&session_25_touch_1, "Touched");
    group.bench_function("session_25_touch_1", move |b| {
        b.iter_batched_ref(
            || PersistEditInput {
                session: session_25_touch_1.clone(),
                tmp: TempDir::new("scenario_persist_edit_session_25_touch_1"),
                flip: false,
            },
            |input| black_box(checksum_persist_edit(input, &touch_25)),
            BatchSize::SmallInput,
        )
    });

    let session_medium_touch_1 = fixtures::session::fixture(fixtures::session::Case::SessionMedium);
    let touch_medium = touch_first_flow_node_spec(&session_medium_touch_1, "Touched");
    group.bench_function("session_medium_touch_1", move |b| {
        b.iter_batched_ref(
            || PersistEditInput {
                session: session_medium_touch_1.clone(),
                tmp: TempDir::new("scenario_persist_edit_session_medium_touch_1"),
                flip: false,
            },
            |input| black_box(checksum_persist_edit(input, &touch_medium)),
            BatchSize::SmallInput,
        )
    });

    // Save into an existing session folder to exercise incremental persistence.
    let session_medium_existing =
        fixtures::session::fixture(fixtures::session::Case::SessionMedium);
    let touch_medium_a = touch_first_flow_node_spec(&session_medium_existing, "TouchedA");
    let touch_medium_b = touch_first_flow_node_spec(&session_medium_existing, "TouchedB");
    group.bench_function("session_medium_touch_1_existing_folder", move |b| {
        b.iter_batched_ref(
            || {
                let input = PersistEditInput {
                    session: session_medium_existing.clone(),
                    tmp: TempDir::new(
                        "scenario_persist_edit_session_medium_touch_1_existing_folder",
                    ),
                    flip: false,
                };

                let folder = SessionFolder::new(input.tmp.path());
                folder.save_session(black_box(&input.session)).expect("save_session");

                input
            },
            |input| {
                let touch = if input.flip { &touch_medium_a } else { &touch_medium_b };
                input.flip = !input.flip;
                black_box(checksum_persist_edit(input, touch))
            },
            BatchSize::LargeInput,
        )
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_scenario
}
criterion_main!(benches);
