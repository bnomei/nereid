// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

pub(super) fn collect_spanned_text(text: &str, spans: &[(usize, usize, usize)]) -> String {
    let lines = text.split('\n').collect::<Vec<_>>();
    let mut out = String::new();

    for &(y, x0, x1) in spans {
        let line = lines.get(y).expect("y in bounds");
        let slice = line.chars().skip(x0).take((x1 - x0) + 1).collect::<String>();
        out.push_str(&slice);
        out.push('\n');
    }

    out
}
