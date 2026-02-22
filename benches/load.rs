// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use nereid::model::Session;
use nereid::store::SessionFolder;

mod fixtures;
mod profiler;

use fixtures::TempDir;

struct SeededFolder {
    _tmp: TempDir,
    folder: SessionFolder,
}

fn seed_folder(prefix: &str, session: &Session) -> SeededFolder {
    let tmp = TempDir::new(prefix);
    let folder = SessionFolder::new(tmp.path());
    folder.save_session(session).expect("save_session");
    folder.flush_ascii_exports();
    SeededFolder { _tmp: tmp, folder }
}

// Benchmark identity (keep stable):
// - Group name in this file: `store.load_session`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `small`, `medium`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn benches_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("store.load_session");

    let session_small = fixtures::session::fixture(fixtures::session::Case::SessionSmall);
    let diagrams_small = session_small.diagrams().len() as u64;
    let seeded_small = seed_folder("store_load_session_small", &session_small);
    group.throughput(Throughput::Elements(diagrams_small));
    group.bench_function("small", move |b| {
        b.iter(|| {
            let loaded = seeded_small.folder.load_session().expect("load_session");
            black_box(fixtures::checksum_session(black_box(&loaded)))
        })
    });

    let session_medium = fixtures::session::fixture(fixtures::session::Case::SessionMedium);
    let diagrams_medium = session_medium.diagrams().len() as u64;
    let seeded_medium = seed_folder("store_load_session_medium", &session_medium);
    group.throughput(Throughput::Elements(diagrams_medium));
    group.bench_function("medium", move |b| {
        b.iter(|| {
            let loaded = seeded_medium.folder.load_session().expect("load_session");
            black_box(fixtures::checksum_session(black_box(&loaded)))
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_load
}
criterion_main!(benches);
