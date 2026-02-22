// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::time::Duration;

use criterion::Criterion;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|raw| raw.trim().parse::<usize>().ok()).unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|raw| raw.trim().parse::<u64>().ok()).unwrap_or(default)
}

pub fn criterion() -> Criterion {
    let sample_size = env_usize("BENCH_SAMPLE_SIZE", 60).clamp(10, 200);
    let warmup_secs = env_u64("BENCH_WARMUP_SECS", 3).clamp(1, 60);
    let measurement_secs = env_u64("BENCH_MEASUREMENT_SECS", 5).clamp(1, 120);

    Criterion::default()
        .sample_size(sample_size)
        .warm_up_time(Duration::from_secs(warmup_secs))
        .measurement_time(Duration::from_secs(measurement_secs))
}
