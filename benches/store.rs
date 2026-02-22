// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use nereid::format::mermaid::{export_flowchart, export_sequence_diagram};
use nereid::layout::{flowchart::layout_flowchart, sequence::layout_sequence};
use nereid::model::{DiagramAst, Session};
use nereid::render::{flowchart::render_flowchart_unicode, sequence::render_sequence_unicode};
use nereid::store::SessionFolder;

mod fixtures;
mod profiler;

use fixtures::TempDir;

fn checksum_compute_only_save(session: &Session) -> u64 {
    let mut acc = 0u64;

    for diagram in session.diagrams().values() {
        match diagram.ast() {
            DiagramAst::Sequence(ast) => {
                let mmd = export_sequence_diagram(black_box(ast)).expect("export_sequence_diagram");
                let layout = layout_sequence(black_box(ast)).expect("layout_sequence");
                let rendered = render_sequence_unicode(black_box(ast), black_box(&layout))
                    .expect("render_sequence_unicode");

                acc = acc.wrapping_mul(131).wrapping_add(mmd.len() as u64);
                acc = acc.wrapping_mul(131).wrapping_add(layout.participant_cols().len() as u64);
                acc = acc.wrapping_mul(131).wrapping_add(layout.messages().len() as u64);
                acc = acc.wrapping_mul(131).wrapping_add(rendered.len() as u64);
            }
            DiagramAst::Flowchart(ast) => {
                let mmd = export_flowchart(black_box(ast)).expect("export_flowchart");
                let layout = layout_flowchart(black_box(ast)).expect("layout_flowchart");
                let rendered = render_flowchart_unicode(black_box(ast), black_box(&layout))
                    .expect("render_flowchart_unicode");

                acc = acc.wrapping_mul(131).wrapping_add(mmd.len() as u64);
                acc = acc.wrapping_mul(131).wrapping_add(layout.layers().len() as u64);
                acc = acc.wrapping_mul(131).wrapping_add(layout.node_placements().len() as u64);
                acc = acc.wrapping_mul(131).wrapping_add(rendered.len() as u64);
            }
        }
    }

    acc
}

// Benchmark identity (keep stable):
// - Group name in this file: `store.save_session`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `compute_only_small`, `io_medium_dense`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn benches_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("store.save_session");

    let session_small = fixtures::session::fixture(fixtures::session::Case::SessionSmall);
    let session_small_compute = session_small.clone();
    group.bench_function("compute_only_small", move |b| {
        b.iter(|| black_box(checksum_compute_only_save(black_box(&session_small_compute))))
    });
    group.bench_function("io_small", move |b| {
        b.iter_batched_ref(
            || TempDir::new("store_save_session_io_small"),
            |tmp| {
                let folder = SessionFolder::new(tmp.path());
                folder.save_session(black_box(&session_small)).expect("save_session");
                black_box(std::fs::metadata(folder.meta_path()).expect("meta_path metadata").len())
            },
            BatchSize::SmallInput,
        )
    });

    let session_medium = fixtures::session::fixture(fixtures::session::Case::SessionMedium);
    let session_medium_compute = session_medium.clone();
    group.bench_function("compute_only_medium", move |b| {
        b.iter(|| black_box(checksum_compute_only_save(black_box(&session_medium_compute))))
    });
    group.bench_function("io_medium", move |b| {
        b.iter_batched_ref(
            || TempDir::new("store_save_session_io_medium"),
            |tmp| {
                let folder = SessionFolder::new(tmp.path());
                folder.save_session(black_box(&session_medium)).expect("save_session");
                black_box(std::fs::metadata(folder.meta_path()).expect("meta_path metadata").len())
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_store
}
criterion_main!(benches);
