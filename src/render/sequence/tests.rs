// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::super::test_utils::collect_spanned_text;
use super::{
    render_sequence_unicode, render_sequence_unicode_annotated,
    render_sequence_unicode_with_options, SELF_MESSAGE_STUB_LEN,
};
use crate::format::mermaid::sequence::parse_sequence_diagram;
use crate::layout::layout_sequence;
use crate::model::ids::ObjectId;
use crate::model::seq_ast::{
    SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
};
use crate::model::{DiagramId, ObjectRef};
use crate::render::{HighlightIndex, RenderOptions};

const DETERMINISM_REPEAT_RUNS: usize = 100;

fn assert_highlight_spans_in_bounds(
    fixture_id: &str,
    text: &str,
    highlight_index: &HighlightIndex,
) {
    let lines = text.split('\n').collect::<Vec<_>>();

    for (object_ref, spans) in highlight_index {
        assert!(
            !spans.is_empty(),
            "sequence fixture `{fixture_id}` produced empty spans for {object_ref}"
        );
        for (span_idx, (y, x0, x1)) in spans.iter().copied().enumerate() {
            let line = lines.get(y).unwrap_or_else(|| {
                panic!(
                    "sequence fixture `{fixture_id}` has out-of-bounds y={y} for {object_ref} span #{span_idx}"
                )
            });
            let line_chars = line.chars().collect::<Vec<_>>();
            let line_len = super::super::text::text_len(line);
            assert!(
                line_len > 0,
                "sequence fixture `{fixture_id}` has span on empty line for {object_ref} span #{span_idx}: (y={y}, x0={x0}, x1={x1})"
            );
            assert!(
                x0 <= x1,
                "sequence fixture `{fixture_id}` has inverted span for {object_ref} span #{span_idx}: (y={y}, x0={x0}, x1={x1})"
            );
            assert!(
                x1 < line_len,
                "sequence fixture `{fixture_id}` has out-of-bounds x1 for {object_ref} span #{span_idx}: (y={y}, x0={x0}, x1={x1}, line_len={line_len})"
            );
            for x in x0..=x1 {
                let ch = *line_chars.get(x).unwrap_or_else(|| {
                    panic!(
                        "sequence fixture `{fixture_id}` missing char cell for {object_ref} span #{span_idx}: (y={y}, x={x}, line_len={line_len})"
                    )
                });
                assert!(
                    ch != ' ',
                    "sequence fixture `{fixture_id}` has non-visible cell in span for {object_ref} span #{span_idx}: (y={y}, x={x})"
                );
            }
        }
    }
}

#[test]
fn snapshot_two_participants_one_message() {
    let mut ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:alice").expect("participant id");
    let p_bob = ObjectId::new("p:bob").expect("participant id");

    ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));

    ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").expect("message id"),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));

    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            " ┌───────┐        ┌─────┐\n │ Alice │        │ Bob │\n └───────┘        └─────┘\n     │               │\n     │               │\n     ├────Hello─────▶│\n     │               │"
        );
}

#[test]
fn snapshot_participant_notes_toggle() {
    let mut ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:alice").expect("participant id");
    let p_bob = ObjectId::new("p:bob").expect("participant id");

    let mut alice = SequenceParticipant::new("Alice");
    alice.set_note(Some("note"));
    ast.participants_mut().insert(p_alice.clone(), alice);
    ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));

    ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").expect("message id"),
        p_alice,
        p_bob,
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));

    let layout = layout_sequence(&ast).expect("layout");

    let notes_off = render_sequence_unicode_with_options(
        &ast,
        &layout,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 0,
        },
    )
    .expect("render");
    assert_eq!(
            notes_off,
            " ┌───────┐        ┌─────┐\n │ Alice │        │ Bob │\n └───────┘        └─────┘\n     │               │\n     │               │\n     ├────Hello─────▶│\n     │               │"
        );

    let notes_on = render_sequence_unicode_with_options(
        &ast,
        &layout,
        RenderOptions { show_notes: true, prefix_object_labels: false, flowchart_extra_col_gap: 0 },
    )
    .expect("render");
    assert_eq!(
            notes_on,
            " ┌───────┐        ┌─────┐\n │ Alice │        │ Bob │\n │ note  │        │     │\n └───────┘        └─────┘\n     │               │\n     │               │\n     ├────Hello─────▶│\n     │               │"
        );
}

#[test]
fn snapshot_alt_else_block_decorations() {
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
alt X\n\
B->>A: In1\n\
else Y\n\
B->>A: In2\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            " ┌───┐        ┌───┐\n │ A │        │ B │\n └───┘        └───┘\n   │            │\n   │            │\n   ├────Pre────▶│\n   │            │\n┌─ALT X─────────│───────┐\n│  │            │       │\n│  │◀────In1────┤       │\n│  │            │       │\n├─ELSE Y────────│───────┤\n│  │            │       │\n│  │◀────In2────┤       │\n└──│────────────│───────┘\n   │            │\n   ├───Post────▶│\n   │            │"
        );
}

#[test]
fn snapshot_opt_block_decorations() {
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
opt Maybe\n\
B->>A: In\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            " ┌───┐        ┌───┐\n │ A │        │ B │\n └───┘        └───┘\n   │            │\n   │            │\n   ├────Pre────▶│\n   │            │\n┌─OPT Maybe─────│───────┐\n│  │            │       │\n│  │◀────In─────┤       │\n└──│────────────│───────┘\n   │            │\n   ├───Post────▶│\n   │            │"
        );
}

#[test]
fn snapshot_loop_block_decorations() {
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
loop R\n\
B->>A: In\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            " ┌───┐        ┌───┐\n │ A │        │ B │\n └───┘        └───┘\n   │            │\n   │            │\n   ├────Pre────▶│\n   │            │\n┌─LOOP R────────│───────┐\n│  │            │       │\n│  │◀────In─────┤       │\n└──│────────────│───────┘\n   │            │\n   ├───Post────▶│\n   │            │"
        );
}

#[test]
fn snapshot_par_and_block_decorations() {
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
par First\n\
A->>B: Left\n\
and Second\n\
B->>A: Right\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            " ┌───┐        ┌───┐\n │ A │        │ B │\n └───┘        └───┘\n   │            │\n   │            │\n   ├────Pre────▶│\n   │            │\n┌─PAR First─────│───────┐\n│  │            │       │\n│  ├───Left────▶│       │\n│  │            │       │\n├─AND Second────│───────┤\n│  │            │       │\n│  │◀───Right───┤       │\n└──│────────────│───────┘\n   │            │\n   ├───Post────▶│\n   │            │"
        );
}

#[test]
fn snapshot_nested_block_decorations() {
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
alt Outer\n\
B->>A: In0\n\
opt Inner\n\
B->>A: In1\n\
end\n\
B->>A: In2\n\
else Other\n\
B->>A: In3\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            " ┌───┐        ┌───┐\n │ A │        │ B │\n └───┘        └───┘\n   │            │\n   │            │\n   ├────Pre────▶│\n   │            │\n┌─ALT Outer─────│───────┐\n│  │            │       │\n│  │◀────In0────┤       │\n│  │            │       │\n│  │            │       │\n│ ││OPT Inner───│─────┐ │\n│ ││◀────In1────┤     │ │\n│ ││────────────│─────┘ │\n│  │            │       │\n│  │◀────In2────┤       │\n│  │            │       │\n├─ELSE Other────│───────┤\n│  │            │       │\n│  │◀────In3────┤       │\n└──│────────────│───────┘\n   │            │\n   ├───Post────▶│\n   │            │"
        );
}

#[test]
fn snapshot_three_participants_two_way_messages() {
    let mut ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:alice").expect("participant id");
    let p_bob = ObjectId::new("p:bob").expect("participant id");
    let p_carol = ObjectId::new("p:carol").expect("participant id");

    ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    ast.participants_mut().insert(p_carol.clone(), SequenceParticipant::new("Carol"));

    ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").expect("message id"),
        p_alice.clone(),
        p_carol.clone(),
        SequenceMessageKind::Async,
        "Ping",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0002").expect("message id"),
        p_carol.clone(),
        p_bob.clone(),
        SequenceMessageKind::Return,
        "Pong",
        2000,
    ));

    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");

    assert_eq!(
            rendered,
            " ┌───────┐        ┌─────┐        ┌───────┐\n │ Alice │        │ Bob │        │ Carol │\n └───────┘        └─────┘        └───────┘\n     │               │               │\n     │               │               │\n     ├─────────────Ping─────────────▷│\n     │               │               │\n     │               │◁─────Pong─────┤\n     │               │               │"
        );
}

#[test]
fn self_message_uses_wider_stub_and_keeps_both_right_corners() {
    let mut ast = SequenceAst::default();
    let p_a = ObjectId::new("p:a").expect("participant id");
    let p_b = ObjectId::new("p:b").expect("participant id");

    ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(p_b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").expect("message id"),
        p_a.clone(),
        p_a,
        SequenceMessageKind::Sync,
        "Hold line",
        1000,
    ));

    let layout = layout_sequence(&ast).expect("layout");
    let rendered = render_sequence_unicode(&ast, &layout).expect("render");
    let lines = rendered.lines().collect::<Vec<_>>();

    let msg_line_idx = lines
        .iter()
        .position(|line| line.contains("Hold"))
        .unwrap_or_else(|| panic!("self-message line in:\n{rendered}"));
    let top_chars = lines[msg_line_idx].chars().collect::<Vec<_>>();
    let top_corner_idx = top_chars.iter().position(|ch| *ch == '┐').expect("top right corner");
    let top_start_idx =
        top_chars.iter().position(|ch| !ch.is_whitespace()).expect("self loop start");
    assert!(top_corner_idx > top_start_idx);
    assert!(top_corner_idx.saturating_sub(top_start_idx) > SELF_MESSAGE_STUB_LEN);
    assert_eq!(top_chars[top_corner_idx.saturating_sub(1)], '─');

    let bottom_chars = lines[msg_line_idx + 1].chars().collect::<Vec<_>>();
    assert!(bottom_chars.contains(&'┘'));
}

#[test]
fn annotated_render_indexes_participants_and_messages() {
    let mut ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:alice").expect("participant id");
    let p_bob = ObjectId::new("p:bob").expect("participant id");

    ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));

    ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").expect("message id"),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));

    let layout = layout_sequence(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-seq").expect("diagram id");
    let annotated = render_sequence_unicode_annotated(&diagram_id, &ast, &layout).expect("render");

    assert_eq!(annotated.text, render_sequence_unicode(&ast, &layout).expect("plain render"));

    let alice_ref: ObjectRef = "d:d-seq/seq/participant/p:alice".parse().expect("object ref");
    let bob_ref: ObjectRef = "d:d-seq/seq/participant/p:bob".parse().expect("object ref");
    let msg_ref: ObjectRef = "d:d-seq/seq/message/m:0001".parse().expect("object ref");

    let alice_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&alice_ref).expect("alice spans"),
    );
    assert!(alice_text.contains("Alice"));

    let bob_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&bob_ref).expect("bob spans"),
    );
    assert!(bob_text.contains("Bob"));

    let msg_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&msg_ref).expect("msg spans"),
    );
    assert!(msg_text.contains("Hello"));
    assert!(msg_text.contains('▶'));
    assert!(msg_text.contains('│'));
}

#[test]
fn annotated_render_indexes_sequence_blocks() {
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
alt Outer\n\
B->>A: In0\n\
else Other\n\
B->>A: In1\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-seq").expect("diagram id");
    let annotated = render_sequence_unicode_annotated(&diagram_id, &ast, &layout).expect("render");

    let block_id = ast.blocks().first().expect("block").block_id().to_string();
    let block_ref: ObjectRef = format!("d:d-seq/seq/block/{block_id}").parse().expect("block ref");

    let spans = annotated.highlight_index.get(&block_ref).expect("block spans");
    assert!(!spans.is_empty());

    let block_text = collect_spanned_text(&annotated.text, spans);
    assert!(block_text.contains("ALT"), "block text did not include ALT label:\n{block_text}");
    assert!(block_text.contains("Outer"), "block text did not include block title:\n{block_text}");
}

#[test]
fn annotated_render_highlights_match_visible_cells_for_participants_messages_blocks_sections() {
    let fixture_id = "nested-visible-highlight-projection";
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
alt Outer\n\
B->>A: In0\n\
opt Inner\n\
B->>A: In1\n\
end\n\
B->>A: In2\n\
else Other\n\
B->>A: In3\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-seq-visible").expect("diagram id");
    let annotated = render_sequence_unicode_annotated(&diagram_id, &ast, &layout).expect("render");

    assert_highlight_spans_in_bounds(fixture_id, &annotated.text, &annotated.highlight_index);

    let participant_a_id = ast
        .participants()
        .iter()
        .find(|(_, participant)| participant.mermaid_name() == "A")
        .map(|(id, _)| id.clone())
        .expect("participant A");
    let pre_msg_id = ast
        .messages()
        .iter()
        .find(|message| message.text() == "Pre")
        .map(|message| message.message_id().clone())
        .expect("Pre message");
    let top_block_id = ast.blocks().first().expect("top block").block_id().clone();
    let else_section_id = ast
        .blocks()
        .first()
        .and_then(|block| block.sections().get(1))
        .map(|section| section.section_id().clone())
        .expect("else section");

    let participant_ref: ObjectRef = format!("d:d-seq-visible/seq/participant/{participant_a_id}")
        .parse()
        .expect("participant ref");
    let message_ref: ObjectRef =
        format!("d:d-seq-visible/seq/message/{pre_msg_id}").parse().expect("message ref");
    let block_ref: ObjectRef =
        format!("d:d-seq-visible/seq/block/{top_block_id}").parse().expect("block ref");
    let section_ref: ObjectRef =
        format!("d:d-seq-visible/seq/section/{else_section_id}").parse().expect("section ref");

    for (kind, object_ref) in [
        ("participant", &participant_ref),
        ("message", &message_ref),
        ("block", &block_ref),
        ("section", &section_ref),
    ] {
        let spans = annotated.highlight_index.get(object_ref).unwrap_or_else(|| {
            panic!("missing {kind} highlight object in fixture `{fixture_id}`: {object_ref}")
        });
        assert!(!spans.is_empty(), "empty {kind} spans for {object_ref}");
    }

    let participant_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&participant_ref).expect("participant spans"),
    );
    assert!(participant_text.contains('A'));

    let message_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&message_ref).expect("message spans"),
    );
    assert!(message_text.contains("Pre"));
    assert!(message_text.contains('▶'));

    let block_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&block_ref).expect("block spans"),
    );
    assert!(block_text.contains("ALT"), "block text did not include ALT label:\n{block_text}");
    assert!(block_text.contains("Outer"), "block text did not include block title:\n{block_text}");

    let section_text = collect_spanned_text(
        &annotated.text,
        annotated.highlight_index.get(&section_ref).expect("section spans"),
    );
    assert!(section_text.contains("ELSE"));
    assert!(section_text.contains("Other"));
}

#[test]
fn repeat_run_render_sequence_unicode_is_deterministic_for_nested_block_fixture() {
    let fixture_id = "nested-alt-opt-else";
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
alt Outer\n\
B->>A: In0\n\
opt Inner\n\
B->>A: In1\n\
end\n\
B->>A: In2\n\
else Other\n\
B->>A: In3\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let baseline = render_sequence_unicode(&ast, &layout).expect("baseline render");

    for run_idx in 1..=DETERMINISM_REPEAT_RUNS {
        let rendered = render_sequence_unicode(&ast, &layout).expect("repeat render");
        assert_eq!(
            rendered, baseline,
            "sequence determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
    }
}

#[test]
fn repeat_run_render_sequence_unicode_annotated_is_deterministic_for_nested_block_fixture() {
    let fixture_id = "nested-alt-opt-else";
    let input = "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
alt Outer\n\
B->>A: In0\n\
opt Inner\n\
B->>A: In1\n\
end\n\
B->>A: In2\n\
else Other\n\
B->>A: In3\n\
end\n\
A->>B: Post\n";

    let ast = parse_sequence_diagram(input).expect("parse");
    let layout = layout_sequence(&ast).expect("layout");
    let diagram_id = DiagramId::new("d-seq-det").expect("diagram id");
    let baseline =
        render_sequence_unicode_annotated(&diagram_id, &ast, &layout).expect("baseline render");

    assert_highlight_spans_in_bounds(fixture_id, &baseline.text, &baseline.highlight_index);

    for run_idx in 1..=DETERMINISM_REPEAT_RUNS {
        let annotated =
            render_sequence_unicode_annotated(&diagram_id, &ast, &layout).expect("repeat render");
        assert_eq!(
            annotated.text, baseline.text,
            "sequence annotated text determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
        assert_eq!(
            annotated.highlight_index, baseline.highlight_index,
            "sequence annotated highlight determinism mismatch for fixture `{fixture_id}` at run {run_idx}"
        );
        assert_highlight_spans_in_bounds(fixture_id, &annotated.text, &annotated.highlight_index);
    }
}
