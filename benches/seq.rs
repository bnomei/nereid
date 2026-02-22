// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use nereid::layout::sequence::{layout_sequence, SequenceLayout};

mod fixtures;
mod profiler;

// Benchmark identity (keep stable):
// - Group name in this file: `seq.layout`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `small`, `medium`, `large_long_text`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn checksum_layout(layout: &SequenceLayout) -> u64 {
    let mut acc = 0u64;

    acc = acc.wrapping_mul(131).wrapping_add(layout.participant_cols().len() as u64);
    acc = acc.wrapping_mul(131).wrapping_add(layout.messages().len() as u64);

    for (participant_id, col) in layout.participant_cols() {
        acc = acc.wrapping_mul(131).wrapping_add(participant_id.as_str().len() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(*col as u64);
    }

    for msg in layout.messages() {
        acc = acc.wrapping_mul(131).wrapping_add(msg.message_id().as_str().len() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(msg.from_col() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(msg.to_col() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(msg.row() as u64);
    }

    acc
}

fn benches_seq(c: &mut Criterion) {
    let mut group = c.benchmark_group("seq.layout");

    for (case_id, ast) in [
        (fixtures::seq::Case::Small.id(), fixtures::seq::fixture(fixtures::seq::Case::Small)),
        (fixtures::seq::Case::Medium.id(), fixtures::seq::fixture(fixtures::seq::Case::Medium)),
        (
            fixtures::seq::Case::LargeLongText.id(),
            fixtures::seq::fixture(fixtures::seq::Case::LargeLongText),
        ),
    ] {
        let messages = ast.messages().len() as u64;
        group.throughput(Throughput::Elements(messages));
        group.bench_function(case_id, move |b| {
            b.iter(|| {
                let layout = layout_sequence(black_box(&ast)).expect("layout_sequence");
                black_box(checksum_layout(black_box(&layout)))
            })
        });
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_seq
}
criterion_main!(benches);
