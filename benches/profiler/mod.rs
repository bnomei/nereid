// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::Criterion;

use pprof::criterion::{Output, PProfProfiler};

fn env_i32(name: &str, default: i32) -> i32 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<i32>().ok())
        .unwrap_or(default)
}

pub fn criterion() -> Criterion {
    let frequency = env_i32("PROFILE_FREQ", 100).clamp(1, 1000);
    Criterion::default().with_profiler(PProfProfiler::new(frequency, Output::Flamegraph(None)))
}
