// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nereid::layout::{flowchart::layout_flowchart, sequence::layout_sequence};
use nereid::model::{Diagram, DiagramAst, DiagramId};
use nereid::render::render_diagram_unicode_annotated;
use nereid::render::{flowchart::render_flowchart_unicode, sequence::render_sequence_unicode};

mod fixtures;
mod profiler;

// Benchmark identity (keep stable):
// - Group names in this file: `render.sequence`, `render.flow`, `render.annotated`,
//   `render.class_er`, `render.gantt`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `small`, `medium_dense`, `large_long_labels`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn benches_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render.sequence");
    let seq_small = fixtures::seq::fixture(fixtures::seq::Case::Small);
    group.bench_function(fixtures::seq::Case::Small.id(), move |b| {
        b.iter(|| {
            let layout = layout_sequence(black_box(&seq_small)).expect("layout_sequence");
            let rendered = render_sequence_unicode(black_box(&seq_small), black_box(&layout))
                .expect("render_sequence_unicode");
            black_box(rendered.len())
        })
    });
    let seq_small_long_text = fixtures::seq::fixture(fixtures::seq::Case::SmallLongText);
    group.bench_function(fixtures::seq::Case::SmallLongText.id(), move |b| {
        b.iter(|| {
            let layout = layout_sequence(black_box(&seq_small_long_text)).expect("layout_sequence");
            let rendered =
                render_sequence_unicode(black_box(&seq_small_long_text), black_box(&layout))
                    .expect("render_sequence_unicode");
            black_box(rendered.len())
        })
    });
    let seq_self_loop_dense = fixtures::seq::fixture(fixtures::seq::Case::SelfLoopDense);
    group.bench_function(fixtures::seq::Case::SelfLoopDense.id(), move |b| {
        b.iter(|| {
            let layout = layout_sequence(black_box(&seq_self_loop_dense)).expect("layout_sequence");
            let rendered =
                render_sequence_unicode(black_box(&seq_self_loop_dense), black_box(&layout))
                    .expect("render_sequence_unicode");
            black_box(rendered.len())
        })
    });
    let seq_nested_blocks = fixtures::seq::fixture(fixtures::seq::Case::NestedBlocks);
    group.bench_function(fixtures::seq::Case::NestedBlocks.id(), move |b| {
        b.iter(|| {
            let layout = layout_sequence(black_box(&seq_nested_blocks)).expect("layout_sequence");
            let rendered =
                render_sequence_unicode(black_box(&seq_nested_blocks), black_box(&layout))
                    .expect("render_sequence_unicode");
            black_box(rendered.len())
        })
    });
    group.finish();

    let mut group = c.benchmark_group("render.flow");
    for case in [
        fixtures::flow::Case::Small,
        fixtures::flow::Case::MediumDense,
        fixtures::flow::Case::DenseCrossing,
        fixtures::flow::Case::LargeLongLabels,
        fixtures::flow::Case::RoutingStress,
    ] {
        let flow = fixtures::flow::fixture(case);
        group.bench_function(case.id(), move |b| {
            b.iter(|| {
                let layout = layout_flowchart(black_box(&flow)).expect("layout_flowchart");
                let rendered = render_flowchart_unicode(black_box(&flow), black_box(&layout))
                    .expect("render_flowchart_unicode");
                black_box(rendered.len())
            })
        });
    }
    group.finish();

    let mut group = c.benchmark_group("render.annotated");
    let annotated_flow = Diagram::new(
        DiagramId::new("bench:render:annotated-flow").expect("diagram id"),
        "Annotated Flow",
        DiagramAst::Flowchart(fixtures::flow::fixture(fixtures::flow::Case::MediumDense)),
    );
    group.bench_function("flow_medium_dense", move |b| {
        b.iter(|| {
            let rendered = render_diagram_unicode_annotated(black_box(&annotated_flow))
                .expect("render_diagram_unicode_annotated");
            black_box(
                rendered
                    .text
                    .len()
                    .wrapping_add(rendered.highlight_index.values().map(Vec::len).sum::<usize>()),
            )
        })
    });
    let annotated_sequence = Diagram::new(
        DiagramId::new("bench:render:annotated-sequence").expect("diagram id"),
        "Annotated Sequence",
        DiagramAst::Sequence(fixtures::seq::fixture(fixtures::seq::Case::SelfLoopDense)),
    );
    group.bench_function("sequence_self_loop_dense", move |b| {
        b.iter(|| {
            let rendered = render_diagram_unicode_annotated(black_box(&annotated_sequence))
                .expect("render_diagram_unicode_annotated");
            black_box(
                rendered
                    .text
                    .len()
                    .wrapping_add(rendered.highlight_index.values().map(Vec::len).sum::<usize>()),
            )
        })
    });
    group.finish();

    let mut group = c.benchmark_group("render.class_er");
    let class_diagram = Diagram::new(
        DiagramId::new("bench:render:class").expect("diagram id"),
        "Class Cyclic Compartments",
        fixtures::graph_track::class_cyclic_compartments(),
    );
    group.bench_function("class_cyclic_compartments", move |b| {
        b.iter(|| {
            let rendered = render_diagram_unicode_annotated(black_box(&class_diagram))
                .expect("render class diagram");
            black_box(
                rendered
                    .text
                    .len()
                    .wrapping_add(rendered.highlight_index.values().map(Vec::len).sum::<usize>()),
            )
        })
    });
    let er_diagram = Diagram::new(
        DiagramId::new("bench:render:er").expect("diagram id"),
        "ER Cyclic Cardinality",
        fixtures::graph_track::er_cyclic_cardinality(),
    );
    group.bench_function("er_cyclic_cardinality", move |b| {
        b.iter(|| {
            let rendered = render_diagram_unicode_annotated(black_box(&er_diagram))
                .expect("render ER diagram");
            black_box(
                rendered
                    .text
                    .len()
                    .wrapping_add(rendered.highlight_index.values().map(Vec::len).sum::<usize>()),
            )
        })
    });
    group.finish();

    let mut group = c.benchmark_group("render.gantt");
    let gantt_diagram = Diagram::new(
        DiagramId::new("bench:render:gantt").expect("diagram id"),
        "Gantt Dependency Dense",
        fixtures::graph_track::gantt_dependency_dense(),
    );
    group.bench_function("dependency_dense", move |b| {
        b.iter(|| {
            let rendered = render_diagram_unicode_annotated(black_box(&gantt_diagram))
                .expect("render gantt diagram");
            black_box(
                rendered
                    .text
                    .len()
                    .wrapping_add(rendered.highlight_index.values().map(Vec::len).sum::<usize>()),
            )
        })
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_render
}
criterion_main!(benches);
