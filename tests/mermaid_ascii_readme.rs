// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::fs;
use std::path::{Path, PathBuf};

use nereid::format::mermaid::{parse_sequence_diagram, MermaidSequenceParseError};
use nereid::layout::sequence::layout_sequence;
use nereid::render::sequence::render_sequence_unicode;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mermaid_ascii_readme")
}

fn read_fixture(name: &str) -> String {
    let path = fixtures_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {path:?}: {err}"))
}

#[test]
fn mermaid_ascii_readme_supported_sequence_cases_render() {
    for case in [
        "seq_simple.mmd",
        "seq_request_response.mmd",
        "seq_three_participants.mmd",
        "seq_self_message.mmd",
    ] {
        let src = read_fixture(case);
        let ast = parse_sequence_diagram(&src).unwrap_or_else(|err| {
            panic!("expected {case} to parse as sequenceDiagram, got error: {err}")
        });
        let layout = layout_sequence(&ast).unwrap_or_else(|err| {
            panic!("expected {case} to layout successfully, got error: {err}")
        });
        let rendered = render_sequence_unicode(&ast, &layout).unwrap_or_else(|err| {
            panic!("expected {case} to render successfully, got error: {err}")
        });
        assert!(!rendered.trim().is_empty(), "expected {case} to render non-empty output");
    }
}

#[test]
#[ignore = "TODO: mermaid-ascii README parity (graph flowcharts + participant aliases + render differences)"]
fn mermaid_ascii_readme_todo_cases_render() {
    let mut failures = Vec::<String>::new();

    // Flowchart examples (upstream uses `graph`; nereid currently only parses `flowchart`).
    for flow_case in [
        "flow_graph_lr_basic.mmd",
        "flow_graph_lr_labeled_edges.mmd",
        "flow_graph_td_labeled_edges.mmd",
        "flow_graph_lr_chain.mmd",
        "flow_graph_lr_colored_classdef.mmd",
    ] {
        failures.push(format!(
            "{flow_case}: TODO (support `graph` header + additional syntax/render parity)"
        ));
    }

    // Sequence aliases (upstream supports `participant A as Alice`; nereid currently rejects it).
    let seq_aliases = read_fixture("seq_aliases.mmd");
    match parse_sequence_diagram(&seq_aliases) {
        Ok(ast) => {
            let layout = layout_sequence(&ast).expect("layout_sequence");
            let rendered = render_sequence_unicode(&ast, &layout).expect("render_sequence_unicode");
            if rendered.trim().is_empty() {
                failures.push("seq_aliases.mmd: rendered empty output".to_owned());
            }
        }
        Err(MermaidSequenceParseError::InvalidParticipantDecl { .. }) => {
            failures.push(
                "seq_aliases.mmd: TODO (support `participant <id> as <name>` syntax)".to_owned(),
            );
        }
        Err(err) => failures.push(format!("seq_aliases.mmd: unexpected parse error: {err}")),
    }

    if !failures.is_empty() {
        panic!("TODO cases remain:\n- {}", failures.join("\n- "));
    }
}
