// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nereid::layout::{flowchart::layout_flowchart, sequence::layout_sequence};
use nereid::render::{flowchart::render_flowchart_unicode, sequence::render_sequence_unicode};

mod fixtures;
mod profiler;

// Benchmark identity (keep stable):
// - Group names in this file: `render.sequence`, `render.flow`
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
    group.finish();

    let mut group = c.benchmark_group("render.flow");
    let flow_small = fixtures::flow::fixture(fixtures::flow::Case::Small);
    group.bench_function(fixtures::flow::Case::Small.id(), move |b| {
        b.iter(|| {
            let layout = layout_flowchart(black_box(&flow_small)).expect("layout_flowchart");
            let rendered = render_flowchart_unicode(black_box(&flow_small), black_box(&layout))
                .expect("render_flowchart_unicode");
            black_box(rendered.len())
        })
    });
    let flow_large_long_labels = fixtures::flow::fixture(fixtures::flow::Case::LargeLongLabels);
    group.bench_function(fixtures::flow::Case::LargeLongLabels.id(), move |b| {
        b.iter(|| {
            let layout =
                layout_flowchart(black_box(&flow_large_long_labels)).expect("layout_flowchart");
            let rendered =
                render_flowchart_unicode(black_box(&flow_large_long_labels), black_box(&layout))
                    .expect("render_flowchart_unicode");
            black_box(rendered.len())
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
