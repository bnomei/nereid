// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use nereid::layout::flowchart::{layout_flowchart, route_flowchart_edges_orthogonal_key_order};

mod fixtures;
mod profiler;

// Benchmark identity (keep stable):
// - Group names in this file: `flow.layout`, `flow.route`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `small`, `medium_dense`, `large_long_labels`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn benches_flow(c: &mut Criterion) {
    {
        let mut group = c.benchmark_group("flow.layout");

        for case in [
            fixtures::flow::Case::Small,
            fixtures::flow::Case::MediumDense,
            fixtures::flow::Case::LargeLongLabels,
            fixtures::flow::Case::DenseCrossing,
            fixtures::flow::Case::RoutingStressWide,
        ] {
            let case_id = case.id();
            let ast = fixtures::flow::fixture(case);
            let nodes = ast.nodes().len() as u64;
            group.throughput(Throughput::Elements(nodes));
            group.bench_function(case_id, move |b| {
                b.iter(|| {
                    let layout = layout_flowchart(black_box(&ast)).expect("layout");
                    black_box(layout.layers().len().wrapping_add(layout.node_placements().len()))
                })
            });
        }

        group.finish();
    }

    {
        let mut group = c.benchmark_group("flow.route");

        for case in [
            fixtures::flow::Case::Small,
            fixtures::flow::Case::MediumDense,
            fixtures::flow::Case::LargeLongLabels,
            fixtures::flow::Case::RoutingStress,
            fixtures::flow::Case::DenseCrossing,
            fixtures::flow::Case::RoutingStressWide,
        ] {
            let case_id = case.id();
            let ast = fixtures::flow::fixture(case);
            let layout = layout_flowchart(&ast).expect("layout");
            let edges = ast.edges().len() as u64;

            group.throughput(Throughput::Elements(edges));
            group.bench_function(case_id, move |b| {
                b.iter(|| {
                    let routes = route_flowchart_edges_orthogonal_key_order(
                        black_box(&ast),
                        black_box(&layout),
                    );

                    let mut acc = 0u64;
                    for points in &routes {
                        acc = acc.wrapping_add(points.len() as u64);
                        for point in points {
                            acc = acc.wrapping_add(point.x().unsigned_abs() as u64);
                            acc = acc.wrapping_add(point.y().unsigned_abs() as u64);
                        }
                    }
                    black_box(acc)
                })
            });
        }

        group.finish();
    }
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_flow
}
criterion_main!(benches);
