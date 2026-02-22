// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use nereid::format::mermaid::{
    export_flowchart, export_sequence_diagram, parse_flowchart, parse_sequence_diagram,
};

mod fixtures;
mod profiler;

// Benchmark identity (keep stable):
// - Group names in this file: `format.parse_flowchart`, `format.parse_sequence`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `small`, `medium_dense`, `large_long_labels`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn benches_parse(c: &mut Criterion) {
    {
        let mut group = c.benchmark_group("format.parse_flowchart");

        for (case_id, ast) in [
            ("small", fixtures::flow::fixture(fixtures::flow::Case::Small)),
            ("medium_dense", fixtures::flow::fixture(fixtures::flow::Case::MediumDense)),
            ("large_long_labels", fixtures::flow::fixture(fixtures::flow::Case::LargeLongLabels)),
        ] {
            let mmd = export_flowchart(&ast).expect("export_flowchart");
            let edges = ast.edges().len() as u64;
            group.throughput(Throughput::Elements(edges));
            group.bench_function(case_id, move |b| {
                b.iter(|| {
                    let parsed = parse_flowchart(black_box(&mmd)).expect("parse_flowchart");
                    black_box(fixtures::checksum_flowchart(black_box(&parsed)))
                })
            });
        }

        group.finish();
    }

    {
        let mut group = c.benchmark_group("format.parse_sequence");

        for (case_id, ast) in [
            (fixtures::seq::Case::Small.id(), fixtures::seq::fixture(fixtures::seq::Case::Small)),
            (fixtures::seq::Case::Medium.id(), fixtures::seq::fixture(fixtures::seq::Case::Medium)),
            (
                fixtures::seq::Case::LargeLongText.id(),
                fixtures::seq::fixture(fixtures::seq::Case::LargeLongText),
            ),
        ] {
            let mmd = export_sequence_diagram(&ast).expect("export_sequence_diagram");
            let messages = ast.messages().len() as u64;
            group.throughput(Throughput::Elements(messages));
            group.bench_function(case_id, move |b| {
                b.iter(|| {
                    let parsed =
                        parse_sequence_diagram(black_box(&mmd)).expect("parse_sequence_diagram");
                    black_box(fixtures::checksum_sequence(black_box(&parsed)))
                })
            });
        }

        group.finish();
    }
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_parse
}
criterion_main!(benches);
