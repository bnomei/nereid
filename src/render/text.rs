// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::Canvas;

pub(crate) fn truncate_with_ellipsis(text: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }

    let len = text_len(text);
    if len <= max_len {
        return text.to_owned();
    }

    if max_len == 1 {
        return "…".to_owned();
    }

    let mut out: String = text.chars().take(max_len - 1).collect();
    out.push('…');
    out
}

pub(crate) fn text_len(text: &str) -> usize {
    text.chars().count()
}

pub(crate) fn canvas_to_string_trimmed(canvas: &Canvas) -> String {
    let mut lines = Vec::<String>::with_capacity(canvas.height());
    for y in 0..canvas.height() {
        let mut line = String::with_capacity(canvas.width());
        for x in 0..canvas.width() {
            // (x, y) is in bounds by construction.
            let ch = canvas.get(x, y).expect("in bounds");
            line.push(ch);
        }

        lines.push(line.trim_end_matches(' ').to_owned());
    }

    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{canvas_to_string_trimmed, text_len, truncate_with_ellipsis};
    use crate::render::Canvas;

    #[test]
    fn truncate_with_ellipsis_handles_small_widths() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "");
        assert_eq!(truncate_with_ellipsis("hello", 1), "…");
        assert_eq!(truncate_with_ellipsis("h", 1), "h");
        assert_eq!(truncate_with_ellipsis("hello", 2), "h…");
    }

    #[test]
    fn truncate_with_ellipsis_counts_chars_not_bytes() {
        assert_eq!(text_len("αβγ"), 3);
        assert_eq!(truncate_with_ellipsis("αβγ", 2), "α…");
    }

    #[test]
    fn canvas_to_string_trimmed_removes_trailing_spaces_and_empty_lines() {
        let mut canvas = Canvas::new(3, 2).expect("canvas");
        canvas.set(0, 0, 'A').expect("set");
        canvas.set(1, 0, ' ').expect("set");
        canvas.set(2, 0, ' ').expect("set");
        assert_eq!(canvas_to_string_trimmed(&canvas), "A");
    }
}
