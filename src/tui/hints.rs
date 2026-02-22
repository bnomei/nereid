// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::HashSet;

pub(crate) fn gen_labels(n: usize, hint_chars: &str) -> Vec<String> {
    let alphabet: Vec<char> = hint_chars.chars().collect();
    assert!(!alphabet.is_empty(), "hint_chars must not be empty");

    let mut seen = HashSet::with_capacity(alphabet.len());
    for &ch in &alphabet {
        if !seen.insert(ch) {
            panic!("hint_chars must not contain duplicate characters");
        }
    }

    let k = alphabet.len();
    if n == 0 {
        return Vec::new();
    }

    if k == 1 {
        let ch = alphabet[0];
        return (1..=n)
            .map(|len| std::iter::repeat(ch).take(len).collect())
            .collect();
    }

    fn pow_saturating(base: usize, exp: usize) -> usize {
        let mut acc = 1usize;
        for _ in 0..exp {
            acc = acc.saturating_mul(base);
        }
        acc
    }

    let mut labels = Vec::with_capacity(n);
    let mut len = 1usize;

    while labels.len() < n {
        let remaining = n - labels.len();
        let count_len = pow_saturating(k, len);
        let to_take = remaining.min(count_len);

        for i in 0..to_take {
            let mut x = i;
            let mut chars = vec![alphabet[0]; len];
            for pos in (0..len).rev() {
                let digit = x % k;
                chars[pos] = alphabet[digit];
                x /= k;
            }
            labels.push(chars.into_iter().collect());
        }

        len += 1;
    }

    labels
}

#[cfg(test)]
mod tests {
    use super::gen_labels;
    use std::collections::HashSet;

    #[test]
    fn gen_labels_n_le_k() {
        assert_eq!(
            gen_labels(3, "abc"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn gen_labels_n_eq_k_plus_one() {
        assert_eq!(
            gen_labels(4, "abc"),
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "aa".to_string()
            ]
        );
    }

    #[test]
    fn gen_labels_fixed_example() {
        let labels = gen_labels(30, "sadfjklewcmpgh");
        assert_eq!(labels.len(), 30);

        let mut uniq = HashSet::with_capacity(labels.len());
        for l in &labels {
            assert!(uniq.insert(l.as_str()), "duplicate label: {l}");
        }

        let expected_first: Vec<String> = ["s", "a", "d", "f", "j"]
            .iter()
            .copied()
            .map(String::from)
            .collect();
        assert_eq!(&labels[..5], expected_first.as_slice());

        let expected_last: Vec<String> = ["sp", "sg", "sh", "as", "aa"]
            .iter()
            .copied()
            .map(String::from)
            .collect();
        assert_eq!(&labels[25..], expected_last.as_slice());
    }

    #[test]
    #[should_panic(expected = "hint_chars must not be empty")]
    fn gen_labels_empty_alphabet_panics() {
        let _ = gen_labels(1, "");
    }

    #[test]
    #[should_panic(expected = "hint_chars must not contain duplicate characters")]
    fn gen_labels_duplicate_chars_panics() {
        let _ = gen_labels(1, "abca");
    }
}
