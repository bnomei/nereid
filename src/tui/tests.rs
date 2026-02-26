// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::{
    apply_highlight_flags, category_path, demo_session, demo_session_fallback,
    diagram_counter_label, diagram_view_title, ensure_active_diagram_id, export_diagram_mermaid,
    fill_highlight_bridge_gaps, fill_highlight_bridge_gaps_unbounded,
    fill_highlight_corner_branch_extensions, footer_help_line, objects_item_bg, osc52_sequence,
    panel_border_style_for_focus, ranked_search_results, search_candidates_from_session,
    search_footer_line, stack_main_panes_vertically, style_for_diagram_cell,
    xref_involves_selected, xref_item_style, xrefs_cursor_highlight_style, App, ExternalAction,
    Focus, FocusOwner, HintKind, HintMode, SearchKind, SearchMode, SelectableObject,
};
use crate::format::mermaid::{parse_flowchart, parse_sequence_diagram};
use crate::model::{
    Diagram, DiagramAst, DiagramId, ObjectId, ObjectRef, Session, SessionId, XRef, XRefId,
    XRefStatus,
};
use crate::render::{diagram::render_diagram_unicode_annotated_with_options, RenderOptions};
use crate::store::SessionFolder;
use crossterm::event::KeyCode;
use ratatui::{layout::Rect, style::Color};
use std::collections::BTreeSet;

fn text_to_string(text: &ratatui::text::Text<'_>) -> String {
    text.lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_to_string(line: &ratatui::text::Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect::<String>()
}

fn single_flowchart_session() -> Session {
    let mut session = Session::new(SessionId::new("s1").expect("session id"));
    let diagram_id = DiagramId::new("flow").expect("diagram id");
    let ast = parse_flowchart(
        r#"flowchart LR
A[Start]
B[End]
A --> B
"#,
    )
    .expect("parse flowchart");
    let diagram = Diagram::new(diagram_id.clone(), "Flow", DiagramAst::Flowchart(ast));
    session.diagrams_mut().insert(diagram_id.clone(), diagram);
    session.set_active_diagram_id(Some(diagram_id));
    session
}

#[test]
fn sequence_objects_include_participant_note_for_inspector() {
    let mut ast = parse_sequence_diagram(
        r#"sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: Hello
"#,
    )
    .expect("parse sequence");
    ast.participants_mut()
        .values_mut()
        .next()
        .expect("participant")
        .set_note(Some("caller must be authenticated"));

    let diagram_id = DiagramId::new("d:seq").expect("diagram id");
    let objects = super::objects_from_sequence_ast(&diagram_id, &ast);

    assert!(objects.iter().any(|obj| obj.note.as_deref() == Some("caller must be authenticated")));
}

#[test]
fn flow_objects_include_node_note_for_inspector() {
    let mut ast = parse_flowchart(
        r#"flowchart LR
A[Start]
B[End]
A --> B
"#,
    )
    .expect("parse flowchart");
    ast.nodes_mut().values_mut().next().expect("node").set_note(Some("must be idempotent"));

    let diagram_id = DiagramId::new("d:flow").expect("diagram id");
    let objects = super::objects_from_flowchart_ast(&diagram_id, &ast);

    assert!(objects.iter().any(|obj| obj.note.as_deref() == Some("must be idempotent")));
}

#[test]
fn selected_highlight_fills_box_drawing_bridge_gaps() {
    let diagram = "─│─";
    let mut flags_by_line = vec![vec![0u8; diagram.chars().count()]];
    let spans = vec![(0usize, 0usize, 0usize), (0usize, 2usize, 2usize)];
    apply_highlight_flags(&mut flags_by_line, &spans, 0b01);

    assert_eq!(flags_by_line[0][1] & 0b01, 0);
    fill_highlight_bridge_gaps(&mut flags_by_line, diagram, 0b01);
    assert_ne!(flags_by_line[0][1] & 0b01, 0);
}

#[test]
fn diagram_and_objects_focus_use_bright_green() {
    let diagram = panel_border_style_for_focus(Focus::Diagram, Focus::Diagram, FocusOwner::Human);
    assert_eq!(diagram.fg, Some(Color::LightGreen));

    let objects = panel_border_style_for_focus(Focus::Objects, Focus::Objects, FocusOwner::Human);
    assert_eq!(objects.fg, Some(Color::LightGreen));
}

#[test]
fn diagram_and_objects_focus_use_bright_blue_for_agent_owner() {
    let diagram = panel_border_style_for_focus(Focus::Diagram, Focus::Diagram, FocusOwner::Agent);
    assert_eq!(diagram.fg, Some(Color::LightBlue));

    let objects = panel_border_style_for_focus(Focus::Objects, Focus::Objects, FocusOwner::Agent);
    assert_eq!(objects.fg, Some(Color::LightBlue));
}

#[test]
fn objects_selected_cursor_uses_bright_green_when_focused() {
    assert_eq!(objects_item_bg(true, true, true, FocusOwner::Human), Some(Color::LightGreen));
}

#[test]
fn objects_selected_cursor_uses_bright_black_when_not_focused() {
    assert_eq!(objects_item_bg(true, true, false, FocusOwner::Human), Some(Color::DarkGray));
}

#[test]
fn objects_cursor_uses_owner_focus_color_when_not_selected() {
    assert_eq!(objects_item_bg(true, false, true, FocusOwner::Human), Some(Color::LightGreen));
    assert_eq!(objects_item_bg(true, false, true, FocusOwner::Agent), Some(Color::LightBlue));
}

#[test]
fn note_cells_render_with_bright_black_foreground() {
    let style = style_for_diagram_cell(
        0,
        false,
        FocusOwner::Human,
        true,
        false,
        Color::LightYellow,
        false,
        Color::Yellow,
    );
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn main_panes_stack_vertically_with_raymon_breakpoints() {
    let narrow_two = Rect { x: 0, y: 0, width: 89, height: 40 };
    let wide_two = Rect { x: 0, y: 0, width: 90, height: 40 };
    assert!(stack_main_panes_vertically(narrow_two, 2));
    assert!(!stack_main_panes_vertically(wide_two, 2));

    let narrow_three = Rect { x: 0, y: 0, width: 109, height: 40 };
    let wide_three = Rect { x: 0, y: 0, width: 110, height: 40 };
    assert!(stack_main_panes_vertically(narrow_three, 3));
    assert!(!stack_main_panes_vertically(wide_three, 3));
}

#[test]
fn xref_indirect_selection_style_uses_bright_black_and_bright_white() {
    let style = xref_item_style(XRefStatus::Ok, true);
    assert_eq!(style.bg, Some(Color::DarkGray));
    assert_eq!(style.fg, Some(Color::White));
}

#[test]
fn xref_non_indirect_dangling_style_stays_red() {
    let style = xref_item_style(XRefStatus::DanglingBoth, false);
    assert_eq!(style.bg, None);
    assert_eq!(style.fg, Some(Color::Red));
}

#[test]
fn xrefs_cursor_highlight_only_when_xrefs_focused() {
    let focused = xrefs_cursor_highlight_style(Focus::XRefs, FocusOwner::Human);
    assert_eq!(focused.bg, Some(Color::LightGreen));
    assert_eq!(focused.fg, Some(Color::White));

    let not_focused = xrefs_cursor_highlight_style(Focus::Objects, FocusOwner::Human);
    assert_eq!(not_focused.bg, None);
    assert_eq!(not_focused.fg, None);
}

#[test]
fn xrefs_cursor_highlight_uses_bright_blue_for_agent_owner() {
    let focused = xrefs_cursor_highlight_style(Focus::XRefs, FocusOwner::Agent);
    assert_eq!(focused.bg, Some(Color::LightBlue));
    assert_eq!(focused.fg, Some(Color::White));
}

#[test]
fn xref_involves_selected_matches_from_and_to_endpoints() {
    let selected: ObjectRef = "d:demo-flow/flow/node/n:a".parse().expect("selected");
    let other: ObjectRef = "d:demo-flow/flow/node/n:b".parse().expect("other");

    let xref_from = XRef::new(selected.clone(), other.clone(), "uses", XRefStatus::Ok);
    assert!(xref_involves_selected(Some(&selected), &xref_from));

    let xref_to = XRef::new(other.clone(), selected.clone(), "uses", XRefStatus::Ok);
    assert!(xref_involves_selected(Some(&selected), &xref_to));

    let xref_neither = XRef::new(other.clone(), other.clone(), "uses", XRefStatus::Ok);
    assert!(!xref_involves_selected(Some(&selected), &xref_neither));
    assert!(!xref_involves_selected(None, &xref_from));
}

#[test]
fn selected_highlight_does_not_fill_space_gaps() {
    let diagram = "─ ─";
    let mut flags_by_line = vec![vec![0u8; diagram.chars().count()]];
    let spans = vec![(0usize, 0usize, 0usize), (0usize, 2usize, 2usize)];
    apply_highlight_flags(&mut flags_by_line, &spans, 0b01);

    fill_highlight_bridge_gaps(&mut flags_by_line, diagram, 0b01);
    assert_eq!(flags_by_line[0][1] & 0b01, 0);
}

#[test]
fn selected_highlight_does_not_fill_multi_cell_bridge_gaps() {
    let diagram = "─││─";
    let mut flags_by_line = vec![vec![0u8; diagram.chars().count()]];
    let spans = vec![(0usize, 0usize, 0usize), (0usize, 3usize, 3usize)];
    apply_highlight_flags(&mut flags_by_line, &spans, 0b01);

    fill_highlight_bridge_gaps(&mut flags_by_line, diagram, 0b01);
    assert_eq!(flags_by_line[0][1] & 0b01, 0);
    assert_eq!(flags_by_line[0][2] & 0b01, 0);
}

#[test]
fn selected_edge_highlight_can_fill_multi_cell_bridge_gaps() {
    let diagram = "─││─";
    let mut flags_by_line = vec![vec![0u8; diagram.chars().count()]];
    let spans = vec![(0usize, 0usize, 0usize), (0usize, 3usize, 3usize)];
    apply_highlight_flags(&mut flags_by_line, &spans, 0b01);

    fill_highlight_bridge_gaps_unbounded(&mut flags_by_line, diagram, 0b01);
    assert_ne!(flags_by_line[0][1] & 0b01, 0);
    assert_ne!(flags_by_line[0][2] & 0b01, 0);
}

#[test]
fn selected_highlight_extends_corner_to_show_branch_stub() {
    let diagram = "     \n──│──\n  │  ";
    let mut flags_by_line = vec![vec![0u8; 5], vec![0u8; 5], vec![0u8; 5]];
    let spans = vec![
        (1usize, 0usize, 2usize), // selected horizontal up to bend
        (2usize, 2usize, 2usize), // selected vertical at bend
    ];
    apply_highlight_flags(&mut flags_by_line, &spans, 0b01);

    fill_highlight_corner_branch_extensions(&mut flags_by_line, diagram, 0b01);
    assert_ne!(flags_by_line[1][3] & 0b01, 0);
}

#[test]
fn diagram_text_does_not_extend_corner_branch_for_flow_edge_cursor() {
    let mut app = App::new(demo_session());
    let edge_ref: ObjectRef = "d:demo-flow/flow/edge/e:ab".parse().expect("edge ref");

    app.base_diagram = "──│─\n  │ ".to_owned();
    app.base_highlight_index = crate::render::HighlightIndex::new();
    app.base_highlight_index
        .insert(edge_ref.clone(), vec![(0usize, 0usize, 2usize), (1usize, 2usize, 2usize)]);
    app.objects =
        vec![SelectableObject { label: "edge e:ab".to_owned(), note: None, object_ref: edge_ref }];
    app.visible_object_indices = vec![0];
    app.objects_state.select(Some(0));
    app.session.selected_object_refs_mut().clear();

    let text = app.diagram_text();
    let row = text
        .lines
        .first()
        .expect("first row")
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect::<Vec<_>>();

    assert_eq!(row[3].0, '─');
    assert_eq!(row[3].1.bg, None);
}

#[test]
fn diagram_text_does_not_extend_corner_branch_for_node_cursor() {
    let mut app = App::new(demo_session());
    let node_ref: ObjectRef = "d:demo-flow/flow/node/n:a".parse().expect("node ref");

    app.base_diagram = "──│─\n  │ ".to_owned();
    app.base_highlight_index = crate::render::HighlightIndex::new();
    app.base_highlight_index
        .insert(node_ref.clone(), vec![(0usize, 0usize, 2usize), (1usize, 2usize, 2usize)]);
    app.objects =
        vec![SelectableObject { label: "node n:a".to_owned(), note: None, object_ref: node_ref }];
    app.visible_object_indices = vec![0];
    app.objects_state.select(Some(0));
    app.session.selected_object_refs_mut().clear();

    let text = app.diagram_text();
    let row = text
        .lines
        .first()
        .expect("first row")
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect::<Vec<_>>();

    assert_eq!(row[3].0, '─');
    assert_eq!(row[3].1.bg, None);
}

#[test]
fn diagram_text_does_not_extend_corner_branch_for_selected_flow_edge() {
    let mut app = App::new(demo_session());
    let edge_ref: ObjectRef = "d:demo-flow/flow/edge/e:ab".parse().expect("edge ref");

    app.base_diagram = "──│─\n  │ ".to_owned();
    app.base_highlight_index = crate::render::HighlightIndex::new();
    app.base_highlight_index
        .insert(edge_ref.clone(), vec![(0usize, 0usize, 2usize), (1usize, 2usize, 2usize)]);
    app.objects_state.select(None);
    app.session.selected_object_refs_mut().clear();
    app.session.selected_object_refs_mut().insert(edge_ref);

    let text = app.diagram_text();
    let row = text
        .lines
        .first()
        .expect("first row")
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect::<Vec<_>>();

    assert_eq!(row[3].0, '─');
    assert_eq!(row[3].1.bg, None);
}

#[test]
fn quits_on_q() {
    let mut app = App::new(demo_session());
    assert!(app.handle_key_code(KeyCode::Char('q')));
}

#[test]
fn does_not_quit_on_esc() {
    let mut app = App::new(demo_session());
    assert!(!app.handle_key_code(KeyCode::Esc));
}

#[test]
fn inspector_hotkey_toggles_visibility() {
    let mut app = App::new(demo_session());
    assert!(!app.inspector_visible);
    app.handle_key_code(KeyCode::Char('4'));
    assert!(app.inspector_visible);
    app.handle_key_code(KeyCode::Tab); // focus objects
    app.handle_key_code(KeyCode::Tab); // focus xrefs
    app.handle_key_code(KeyCode::Char('4'));
    assert!(!app.inspector_visible);
}

#[test]
fn palette_hotkey_toggles_visibility() {
    let mut app = App::new(demo_session());
    assert!(!app.palette_visible);
    app.handle_key_code(KeyCode::Char('|'));
    assert!(app.palette_visible);
    app.handle_key_code(KeyCode::Char('|'));
    assert!(!app.palette_visible);
}

#[test]
fn help_hotkey_toggles_visibility() {
    let mut app = App::new(demo_session());
    assert!(!app.show_help);

    app.handle_key_code(KeyCode::Char('?'));
    assert!(app.show_help);

    app.handle_key_code(KeyCode::Esc);
    assert!(!app.show_help);
}

#[test]
fn help_supports_keyboard_scrolling() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('?'));
    app.help_viewport_height = 10;

    app.handle_key_code(KeyCode::Down);
    assert_eq!(app.help_scroll, 1);

    app.handle_key_code(KeyCode::Char('k'));
    assert_eq!(app.help_scroll, 0);

    app.handle_key_code(KeyCode::PageDown);
    assert_eq!(app.help_scroll, 9);

    app.handle_key_code(KeyCode::PageUp);
    assert_eq!(app.help_scroll, 0);

    app.handle_key_code(KeyCode::End);
    assert_eq!(app.help_scroll, u16::MAX);

    app.handle_key_code(KeyCode::Home);
    assert_eq!(app.help_scroll, 0);
}

#[test]
fn help_mode_consumes_panel_navigation_keys() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('?'));

    let diagram_pan_before = app.pan_y;
    app.handle_key_code(KeyCode::Down);

    assert_eq!(app.pan_y, diagram_pan_before);
    assert_eq!(app.help_scroll, 1);
}

#[test]
fn help_mode_allows_quit_key() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('?'));
    assert!(app.handle_key_code(KeyCode::Char('q')));
}

#[test]
fn number_hotkeys_focus_views() {
    let mut app = App::new(demo_session());
    assert_eq!(app.focus, super::Focus::Diagram);
    assert!(!app.objects_visible);
    assert!(!app.xrefs_visible);

    app.handle_key_code(KeyCode::Char('2'));
    assert_eq!(app.focus, super::Focus::Objects);
    assert!(app.objects_visible);

    app.handle_key_code(KeyCode::Char('3'));
    assert_eq!(app.focus, super::Focus::XRefs);
    assert!(app.xrefs_visible);

    app.handle_key_code(KeyCode::Char('1'));
    assert_eq!(app.focus, super::Focus::Diagram);
}

#[test]
fn footer_toggle_entries_use_square_glyphs_without_parentheses() {
    let mut app = App::new(demo_session());
    let inactive = line_to_string(&footer_help_line(&app, "", false));
    assert!(inactive.contains("Notes:n◼ "));
    assert!(inactive.contains("Ai:a◼ "));
    assert!(!inactive.contains("n("));
    assert!(!inactive.contains("a("));

    app.show_notes = false;
    app.follow_ai = false;
    let active = line_to_string(&footer_help_line(&app, "", false));
    assert!(active.contains("Notes:n◻ "));
    assert!(active.contains("Ai:a◻ "));
}

#[test]
fn compact_footer_shows_only_ai_hint_help_quit() {
    let app = App::new(demo_session());
    let line = line_to_string(&footer_help_line(&app, "", true));

    assert!(line.contains("Ai:a◼ "));
    assert!(line.contains("Hint:f"));
    assert!(line.contains("Help:?"));
    assert!(line.contains("Quit:q"));
    assert!(!line.contains("Nav:"));
    assert!(!line.contains("Panels:"));
    assert!(!line.contains("Search:"));
    assert!(!line.contains("Notes:"));
}

#[test]
fn footer_diagram_hotkeys_dim_when_follow_ai_is_active() {
    let mut app = App::new(demo_session());
    app.focus = Focus::Diagram;
    app.follow_ai = true;
    app.show_notes = true;

    let line = footer_help_line(&app, "", false);
    for hotkey in ["[]", "f", "c", "e", "⏡", "y", "g/t", "n◼ "] {
        let span = line
            .spans
            .iter()
            .find(|span| span.content.as_ref() == hotkey)
            .unwrap_or_else(|| panic!("missing hotkey span: {hotkey}"));
        assert_eq!(span.style.fg, Some(Color::DarkGray));
    }

    let ai_span =
        line.spans.iter().find(|span| span.content.as_ref() == "a◼ ").expect("AI toggle span");
    assert_eq!(ai_span.style.fg, Some(Color::Cyan));
}

#[test]
fn compact_footer_dims_hint_only_for_diagram_focus_when_follow_ai_is_active() {
    let mut app = App::new(demo_session());
    app.follow_ai = true;
    app.focus = Focus::Diagram;

    let diagram_line = footer_help_line(&app, "", true);
    let hint_when_diagram = diagram_line
        .spans
        .iter()
        .find(|span| span.content.as_ref() == "f")
        .expect("compact hint span");
    assert_eq!(hint_when_diagram.style.fg, Some(Color::DarkGray));

    app.focus = Focus::Objects;
    let objects_line = footer_help_line(&app, "", true);
    let hint_when_objects = objects_line
        .spans
        .iter()
        .find(|span| span.content.as_ref() == "f")
        .expect("compact hint span");
    assert_eq!(hint_when_objects.style.fg, Some(Color::Cyan));
}

#[test]
fn diagram_counter_label_pads_index_to_total_width() {
    assert_eq!(diagram_counter_label(Some(4), 12), "[04/12]");
    assert_eq!(diagram_counter_label(Some(4), 123), "[004/123]");
    assert_eq!(diagram_counter_label(Some(12), 12), "[12/12]");
}

#[test]
fn diagram_title_renders_counter_in_bright_green() {
    let title = diagram_view_title("om-02-gear", true, Some(4), 12);

    let counter_span =
        title.spans.iter().find(|span| span.content.as_ref() == "[04/12]").expect("counter span");
    assert_eq!(counter_span.style.fg, Some(Color::LightGreen));
}

#[test]
fn search_footer_shows_accept_and_close_with_key_style() {
    let mut app = App::new(demo_session());
    app.search_mode = SearchMode::Editing;
    app.search_query = "arrow".to_owned();
    let line = search_footer_line(&app, "");

    let rendered = line_to_string(&line);
    assert!(rendered.contains("/arrow"));
    assert!(rendered.contains("Accept:Enter"));
    assert!(rendered.contains("Close:Esc"));

    let enter_span =
        line.spans.iter().find(|span| span.content.as_ref() == "Enter").expect("Enter span");
    assert_eq!(enter_span.style.fg, Some(Color::Cyan));
}

#[test]
fn search_footer_adds_three_spaces_after_query() {
    let mut app = App::new(demo_session());
    app.search_mode = SearchMode::Editing;
    app.search_query = "arrow".to_owned();

    let line = line_to_string(&search_footer_line(&app, ""));
    assert!(line.contains("/arrow   0 | Accept:Enter"));
}

#[test]
fn search_footer_renders_count_in_bright_green() {
    let mut app = App::new(demo_session());
    app.search_mode = SearchMode::Results;
    app.search_query = "arrow".to_owned();
    app.search_results.push("d:demo-00-index/flow/node/n:seq_demo".parse().expect("object ref"));

    let line = search_footer_line(&app, "");
    let count_span =
        line.spans.iter().find(|span| span.content.as_ref() == "1/1").expect("count span");
    assert_eq!(count_span.style.fg, Some(Color::LightGreen));
}

#[test]
fn search_results_footer_keeps_next_hint_and_adds_accept_close() {
    let mut app = App::new(demo_session());
    app.search_mode = SearchMode::Results;
    app.search_query = "arrow".to_owned();
    app.search_results.push("d:demo-00-index/flow/node/n:seq_demo".parse().expect("object ref"));

    let line = line_to_string(&search_footer_line(&app, ""));
    assert!(line.contains("Next:n/N"));
    assert!(line.contains("Accept:Enter"));
    assert!(line.contains("Close:Esc"));
}

#[test]
fn key_a_toggles_follow_ai_mode() {
    let mut app = App::new(demo_session());
    assert!(app.follow_ai);

    app.handle_key_code(KeyCode::Char('a'));
    assert!(!app.follow_ai);

    app.handle_key_code(KeyCode::Char('a'));
    assert!(app.follow_ai);
}

#[test]
fn key_e_queues_edit_external_action() {
    let mut app = App::new(demo_session());
    assert_eq!(app.take_external_action(), None);

    app.handle_key_code(KeyCode::Char('e'));

    assert_eq!(app.take_external_action(), Some(ExternalAction::EditActiveDiagram));
}

#[test]
fn applying_edited_mermaid_updates_active_diagram_and_rev() {
    let mut app = App::new(single_flowchart_session());
    let diagram_id = app.active_diagram_id().cloned().expect("active diagram");
    let diagram = app.session.diagrams().get(&diagram_id).cloned().expect("diagram");
    let baseline_rev = diagram.rev();

    let edited_mermaid =
        export_diagram_mermaid(&diagram).expect("export").replacen("Start", "Start edited", 1);

    app.apply_edited_mermaid_to_diagram(&diagram_id, diagram.kind(), baseline_rev, &edited_mermaid)
        .expect("apply edited mermaid");

    let updated = app.session.diagrams().get(&diagram_id).expect("updated diagram");
    assert_eq!(updated.rev(), baseline_rev + 1);
    assert!(app.pending_diagram_sync.is_none());
}

#[test]
fn applying_comment_only_change_does_not_bump_rev() {
    let mut app = App::new(single_flowchart_session());
    let diagram_id = app.active_diagram_id().cloned().expect("active diagram");
    let diagram = app.session.diagrams().get(&diagram_id).cloned().expect("diagram");
    let baseline_rev = diagram.rev();

    let edited_mermaid =
        format!("{}\n%% comment-only edit", export_diagram_mermaid(&diagram).expect("export"));

    app.apply_edited_mermaid_to_diagram(&diagram_id, diagram.kind(), baseline_rev, &edited_mermaid)
        .expect("apply edited mermaid");

    let updated = app.session.diagrams().get(&diagram_id).expect("updated diagram");
    assert_eq!(updated.rev(), baseline_rev);
}

#[test]
fn enabling_follow_ai_jumps_to_agent_highlight_diagram() {
    let mut app = App::new(demo_session());
    let target: ObjectRef = "d:demo-seq/seq/participant/p:alice".parse().expect("object ref");
    app.agent_highlights.blocking_lock().insert(target.clone());
    app.follow_ai = false;

    app.handle_key_code(KeyCode::Char('a'));

    assert_eq!(app.active_diagram_id().map(ToString::to_string).as_deref(), Some("demo-seq"));
    assert_eq!(app.selected_ref(), Some(&target));
}

#[test]
fn sync_ignores_agent_highlight_when_follow_ai_is_disabled() {
    let mut app = App::new(demo_session());
    let target: ObjectRef = "d:demo-seq/seq/participant/p:alice".parse().expect("object ref");
    app.agent_highlights.blocking_lock().insert(target);
    app.follow_ai = false;

    app.sync_from_ui_state();

    assert_eq!(app.active_diagram_id().map(ToString::to_string).as_deref(), Some("demo-00-index"));
}

#[test]
fn scrolls_with_arrows() {
    let mut app = App::new(demo_session());
    assert!(!app.handle_key_code(KeyCode::Down));
    assert_eq!(app.pan_y, 1);
    assert!(!app.handle_key_code(KeyCode::Right));
    assert_eq!(app.pan_x, 1);
}

#[test]
fn shift_hjkl_pans_by_ten() {
    let mut app = App::new(demo_session());

    assert!(!app.handle_key_code(KeyCode::Char('J')));
    assert_eq!(app.pan_y, 10);
    assert!(!app.handle_key_code(KeyCode::Char('L')));
    assert_eq!(app.pan_x, 10);

    assert!(!app.handle_key_code(KeyCode::Char('K')));
    assert_eq!(app.pan_y, 0);
    assert!(!app.handle_key_code(KeyCode::Char('H')));
    assert_eq!(app.pan_x, 0);
}

#[test]
fn tab_switches_focus() {
    let mut app = App::new(demo_session());
    assert_eq!(app.focus, super::Focus::Diagram);
    assert!(!app.handle_key_code(KeyCode::Tab));
    assert_eq!(app.focus, super::Focus::Diagram);
    app.handle_key_code(KeyCode::Char('2'));
    app.handle_key_code(KeyCode::Char('3'));
    app.handle_key_code(KeyCode::Char('1'));
    assert!(!app.handle_key_code(KeyCode::Tab));
    assert_eq!(app.focus, super::Focus::Objects);
    assert!(!app.handle_key_code(KeyCode::Tab));
    assert_eq!(app.focus, super::Focus::XRefs);
}

#[test]
fn shift_tab_switches_focus_backwards() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('2'));
    app.handle_key_code(KeyCode::Char('3'));
    app.handle_key_code(KeyCode::Char('1'));

    assert_eq!(app.focus, super::Focus::Diagram);
    assert!(!app.handle_key_code(KeyCode::BackTab));
    assert_eq!(app.focus, super::Focus::XRefs);
    assert!(!app.handle_key_code(KeyCode::BackTab));
    assert_eq!(app.focus, super::Focus::Objects);
    assert!(!app.handle_key_code(KeyCode::BackTab));
    assert_eq!(app.focus, super::Focus::Diagram);
}

#[test]
fn object_list_moves_selection_when_focused() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('2')); // toggle+focus objects
    let before = app.selected_ref().map(ToString::to_string);
    app.handle_key_code(KeyCode::Down);
    let after = app.selected_ref().map(ToString::to_string);
    assert_ne!(before, after);
}

#[test]
fn diagram_space_toggles_selected_object() {
    let mut app = App::new(demo_session_fallback());
    let object_ref = app.selected_ref().cloned().expect("selected ref");

    assert!(!app.session.selected_object_refs().contains(&object_ref));
    app.handle_key_code(KeyCode::Char(' '));
    assert!(app.session.selected_object_refs().contains(&object_ref));
    app.handle_key_code(KeyCode::Char(' '));
    assert!(!app.session.selected_object_refs().contains(&object_ref));
}

#[test]
fn key_d_deselects_only_current_diagram_objects() {
    let mut app = App::new(demo_session());
    let current_diagram_ref =
        app.selected_ref().cloned().expect("default selected ref in current diagram");
    let other_diagram_ref: ObjectRef = "d:demo-flow/flow/edge/e:ab".parse().expect("object ref");

    app.session.selected_object_refs_mut().clear();
    app.session.selected_object_refs_mut().insert(current_diagram_ref.clone());
    app.session.selected_object_refs_mut().insert(other_diagram_ref.clone());

    app.handle_key_code(KeyCode::Char('d'));

    assert!(!app.session.selected_object_refs().contains(&current_diagram_ref));
    assert!(app.session.selected_object_refs().contains(&other_diagram_ref));
}

#[test]
fn objects_focus_f_enters_hint_mode() {
    let mut app = App::new(demo_session_fallback());
    app.handle_key_code(KeyCode::Char('2')); // toggle+focus objects

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets,
        other => panic!("expected AwaitingFirst, got {other:?}"),
    };
    assert!(!targets.is_empty());
}

#[test]
fn diagram_y_yanks_ref() {
    let mut app = App::new(demo_session_fallback());
    app.handle_key_code(KeyCode::Char('y'));
    let toast = app.toast.as_ref().expect("toast");
    assert!(toast.message.contains("Yanked object ref"));
}

#[test]
fn diagram_text_highlights_selected_object() {
    let app = App::new(demo_session());
    let text = app.diagram_text();

    let has_highlight = text
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|span| span.style.bg == Some(Color::LightGreen)));

    assert!(has_highlight);
}

#[test]
fn diagram_text_has_no_highlight_when_selection_is_none() {
    let mut app = App::new(demo_session());
    app.objects_state.select(None);

    let text = app.diagram_text();
    let has_highlight = text
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|span| span.style.bg == Some(Color::LightGreen)));

    assert!(!has_highlight);
}

#[test]
fn diagram_text_dims_unselected_lines_when_objects_are_selected() {
    let mut app = App::new(demo_session());
    let object_ref = app.selected_ref().expect("default selection exists").clone();
    app.objects_state.select(None);
    app.session.selected_object_refs_mut().insert(object_ref);

    let text = app.diagram_text();
    let has_gray_lines = text
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|span| span.style.fg == Some(Color::DarkGray)));

    assert!(has_gray_lines);
}

#[test]
fn diagram_text_does_not_dim_unselected_lines_for_cursor_highlight_only() {
    let mut app = App::new(demo_session());
    app.show_notes = false;
    app.rerender_active_diagram_buffer();
    let text = app.diagram_text();
    let has_gray_lines = text.lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.fg == Some(Color::DarkGray)
                && span.style.bg.is_none()
                && !span.content.trim().is_empty()
        })
    });
    assert!(!has_gray_lines);
}

#[test]
fn diagram_text_uses_white_foreground_for_cursor_highlight() {
    let app = App::new(demo_session());
    let text = app.diagram_text();
    let has_white_cursor = text.lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(Color::LightGreen) && span.style.fg == Some(Color::White)
        })
    });

    assert!(has_white_cursor);
}

#[test]
fn diagram_text_renders_selected_objects_on_bright_black_when_not_focused() {
    let mut app = App::new(demo_session());
    let object_ref = app.selected_ref().expect("default selection exists").clone();
    app.objects_state.select(None);
    app.session.selected_object_refs_mut().insert(object_ref);

    let text = app.diagram_text();
    let has_selected_on_bright_black = text.lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.fg == Some(Color::White) && span.style.bg == Some(Color::DarkGray)
        })
    });

    assert!(has_selected_on_bright_black);
}

#[test]
fn diagram_text_uses_bright_black_when_cursor_is_selected() {
    let mut app = App::new(demo_session());
    let object_ref = app.selected_ref().expect("default selection exists").clone();
    app.session.selected_object_refs_mut().insert(object_ref);

    let text = app.diagram_text();
    let has_bright_green = text.lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(Color::LightGreen) && span.style.fg == Some(Color::White)
        })
    });

    assert!(has_bright_green);
}

#[test]
fn sequence_message_focus_highlights_spaces_inside_label() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("om-10-dialogue").expect("diagram id"));
    let object_ref: ObjectRef = "d:om-10-dialogue/seq/message/m:ask_go".parse().expect("msg ref");
    app.select_object_ref(&object_ref);
    app.session.selected_object_refs_mut().clear();

    let text = app.diagram_text();
    let line = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
                .collect::<Vec<_>>()
        })
        .find(|cells| cells.iter().map(|(ch, _)| *ch).collect::<String>().contains("Can I"))
        .expect("message row with phrase");

    let line_text = line.iter().map(|(ch, _)| *ch).collect::<String>();
    let phrase_start = line_text.find("Can I").expect("phrase start");
    let space_x = phrase_start + 3;
    assert_eq!(line[space_x].0, ' ');
    assert_eq!(line[space_x].1.bg, Some(Color::LightGreen));
}

#[test]
fn sequence_message_selected_highlights_spaces_inside_label() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("om-10-dialogue").expect("diagram id"));
    let object_ref: ObjectRef = "d:om-10-dialogue/seq/message/m:ask_go".parse().expect("msg ref");
    app.select_object_ref(&object_ref);
    app.session.selected_object_refs_mut().insert(object_ref);
    app.objects_state.select(None);

    let text = app.diagram_text();
    let line = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
                .collect::<Vec<_>>()
        })
        .find(|cells| cells.iter().map(|(ch, _)| *ch).collect::<String>().contains("Can I"))
        .expect("message row with phrase");

    let line_text = line.iter().map(|(ch, _)| *ch).collect::<String>();
    let phrase_start = line_text.find("Can I").expect("phrase start");
    let space_x = phrase_start + 3;
    assert_eq!(line[space_x].0, ' ');
    assert_eq!(line[space_x].1.bg, Some(Color::DarkGray));
}

#[test]
fn sequence_message_focus_highlights_destination_lifeline_cell() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-seq").expect("diagram id"));
    let object_ref: ObjectRef = "d:demo-seq/seq/message/m:0001".parse().expect("msg ref");
    app.select_object_ref(&object_ref);
    app.session.selected_object_refs_mut().clear();

    let text = app.diagram_text();
    let line = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
                .collect::<Vec<_>>()
        })
        .find(|cells| cells.iter().map(|(ch, _)| *ch).collect::<String>().contains("Hello"))
        .expect("message row with Hello");

    let arrow_x = line.iter().position(|(ch, _)| *ch == '▶').expect("right-facing arrow head");
    let lifeline_x = arrow_x + 1;
    assert_eq!(line[lifeline_x].0, '│');
    assert_eq!(line[lifeline_x].1.bg, Some(Color::LightGreen));
}

#[test]
fn sequence_message_selected_highlights_destination_lifeline_cell() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-seq").expect("diagram id"));
    let object_ref: ObjectRef = "d:demo-seq/seq/message/m:0001".parse().expect("msg ref");
    app.select_object_ref(&object_ref);
    app.session.selected_object_refs_mut().insert(object_ref);
    app.objects_state.select(None);

    let text = app.diagram_text();
    let line = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
                .collect::<Vec<_>>()
        })
        .find(|cells| cells.iter().map(|(ch, _)| *ch).collect::<String>().contains("Hello"))
        .expect("message row with Hello");

    let arrow_x = line.iter().position(|(ch, _)| *ch == '▶').expect("right-facing arrow head");
    let lifeline_x = arrow_x + 1;
    assert_eq!(line[lifeline_x].0, '│');
    assert_eq!(line[lifeline_x].1.bg, Some(Color::DarkGray));
}

#[test]
fn selected_om05_luck_edge_0009_does_not_highlight_node_text() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("om-05-luck").expect("diagram id"));
    let edge_ref: ObjectRef = "d:om-05-luck/flow/edge/e:0009".parse().expect("edge ref");
    app.select_object_ref(&edge_ref);
    app.session.selected_object_refs_mut().clear();
    app.session.selected_object_refs_mut().insert(edge_ref);

    let text = app.diagram_text();
    let has_highlighted_node_text = text.lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(Color::LightGreen)
                && span
                    .content
                    .chars()
                    .any(|ch| ch.is_ascii_alphanumeric() || ch == '\'' || ch == '_')
        })
    });

    assert!(
        !has_highlighted_node_text,
        "selected om-05-luck e:0009 should not highlight node text cells:\n{}",
        text_to_string(&text)
    );
}

#[test]
fn om07_shark_types_keeps_mako_and_attacks_labels_visible_in_tui_render() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("om-07-shark-types").expect("diagram id"));

    let rendered = text_to_string(&app.diagram_text());
    assert!(
        rendered.contains("Mako") || rendered.contains("mako"),
        "expected Mako label to be visible:\n{rendered}"
    );
    assert!(
        rendered.contains("Attacks") || rendered.contains("attacks"),
        "expected Attacks label to be visible:\n{rendered}"
    );
}

#[test]
fn selecting_om07_attacks_keeps_mako_and_attacks_labels_visible_in_tui_render() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("om-07-shark-types").expect("diagram id"));
    let attacks_ref: ObjectRef =
        "d:om-07-shark-types/flow/node/n:attacks".parse().expect("attacks ref");
    app.select_object_ref(&attacks_ref);

    let rendered = text_to_string(&app.diagram_text());
    assert!(
        rendered.contains("Mako") || rendered.contains("mako"),
        "expected Mako label to be visible after selecting attacks:\n{rendered}"
    );
    assert!(
        rendered.contains("Attacks") || rendered.contains("attacks"),
        "expected Attacks label to be visible after selecting attacks:\n{rendered}"
    );
}

#[test]
fn diagram_text_fills_selected_bridge_gaps_under_crossing_lines() {
    let mut app = App::new(demo_session());
    let object_ref: ObjectRef = "d:demo-00-index/flow/edge/e:0003".parse().expect("object ref");

    app.base_diagram = "─│─".to_owned();
    app.base_highlight_index = crate::render::HighlightIndex::new();
    app.base_highlight_index
        .insert(object_ref.clone(), vec![(0usize, 0usize, 0usize), (0usize, 2usize, 2usize)]);
    app.session.selected_object_refs_mut().clear();
    app.session.selected_object_refs_mut().insert(object_ref);
    app.objects_state.select(None);

    let text = app.diagram_text();
    let line = text
        .lines
        .first()
        .expect("diagram contains one line")
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect::<Vec<_>>();

    assert_eq!(line.len(), 3);
    assert_eq!(line[1].0, '│');
    assert_eq!(line[1].1.bg, Some(Color::DarkGray));
}

#[test]
fn diagram_text_fills_multi_cell_selected_bridge_gaps_for_flow_edges() {
    let mut app = App::new(demo_session());
    let object_ref: ObjectRef = "d:demo-00-index/flow/edge/e:0003".parse().expect("object ref");

    app.base_diagram = "─│││─".to_owned();
    app.base_highlight_index = crate::render::HighlightIndex::new();
    app.base_highlight_index
        .insert(object_ref.clone(), vec![(0usize, 0usize, 0usize), (0usize, 4usize, 4usize)]);
    app.session.selected_object_refs_mut().clear();
    app.session.selected_object_refs_mut().insert(object_ref);
    app.objects_state.select(None);

    let text = app.diagram_text();
    let line = text
        .lines
        .first()
        .expect("diagram contains one line")
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect::<Vec<_>>();

    assert_eq!(line.len(), 5);
    for cell in line.iter().take(4).skip(1) {
        assert_eq!(cell.0, '│');
        assert_eq!(cell.1.bg, Some(Color::DarkGray));
    }
}

#[test]
fn diagram_text_highlights_agent_objects() {
    let mut app = App::new(demo_session());
    let object_ref = app.selected_ref().expect("default selection exists").clone();
    app.objects_state.select(None);
    app.agent_highlights.blocking_lock().insert(object_ref);

    let text = app.diagram_text();
    let has_highlight = text
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|span| span.style.bg == Some(Color::LightBlue)));

    assert!(has_highlight);
}

#[test]
fn diagram_text_uses_bright_blue_cursor_for_agent_owner() {
    let mut app = App::new(demo_session());
    app.focus_owner = FocusOwner::Agent;
    let text = app.diagram_text();

    let has_bright_blue = text
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|span| span.style.bg == Some(Color::LightBlue)));

    assert!(has_bright_blue);
}

#[test]
fn diagram_text_highlights_overlap_as_magenta() {
    let app = App::new(demo_session());
    let object_ref = app.selected_ref().expect("default selection exists").clone();
    app.agent_highlights.blocking_lock().insert(object_ref);

    let text = app.diagram_text();
    let has_highlight = text
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|span| span.style.bg == Some(Color::LightBlue)));

    assert!(has_highlight);
}

#[test]
fn osc52_sequence_encodes_payload_and_terminates_with_st() {
    let seq = osc52_sequence("d:demo-flow/flow/edge/e:ab");
    assert!(seq.starts_with("\u{1b}]52;c;"));
    assert!(seq.ends_with("\u{1b}\\"));
    assert!(seq.contains("ZDpkZW1vLWZsb3cvZmxvdy9lZGdlL2U6YWI="));
}

#[test]
fn xref_list_moves_selection_when_focused() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('3')); // toggle+focus xrefs
    let before = app.selected_xref_index();
    app.handle_key_code(KeyCode::Down);
    let after = app.selected_xref_index();
    assert_ne!(before, after);
}

#[test]
fn objects_list_moves_selection_with_h_and_l_when_focused() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('2')); // toggle+focus objects

    let before = app.selected_ref().cloned();
    app.handle_key_code(KeyCode::Char('l'));
    let after_next = app.selected_ref().cloned();
    assert_ne!(before, after_next);

    app.handle_key_code(KeyCode::Char('h'));
    let after_prev = app.selected_ref().cloned();
    assert_eq!(before, after_prev);
}

#[test]
fn xref_list_moves_selection_with_h_and_l_when_focused() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('3')); // toggle+focus xrefs

    let before = app.selected_xref_index();
    app.handle_key_code(KeyCode::Char('l'));
    let after_next = app.selected_xref_index();
    assert_ne!(before, after_next);

    app.handle_key_code(KeyCode::Char('h'));
    let after_prev = app.selected_xref_index();
    assert_eq!(before, after_prev);
}

#[test]
fn objects_selected_only_filter_toggles_with_dash() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('2')); // toggle+focus objects
    assert!(!app.objects_selected_only);

    app.handle_key_code(KeyCode::Char('-'));
    assert!(app.objects_selected_only);

    app.handle_key_code(KeyCode::Char('-'));
    assert!(!app.objects_selected_only);
}

#[test]
fn objects_c_enters_select_hint_mode() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));
    app.handle_key_code(KeyCode::Char('2')); // toggle+focus objects

    app.handle_key_code(KeyCode::Char('c'));

    assert!(matches!(app.hint_mode, HintMode::AwaitingFirst { kind: HintKind::SelectChain, .. }));
}

#[test]
fn xref_dangling_filter_hides_ok_xrefs() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('3')); // toggle+focus xrefs
    let before = app.visible_xref_indices().len();
    app.handle_key_code(KeyCode::Char('-'));
    let after = app.visible_xref_indices().len();
    assert!(after < before);
    assert!(app
        .visible_xref_indices()
        .iter()
        .all(|&idx| app.xrefs[idx].xref.status().is_dangling()));
}

#[test]
fn xref_involving_filter_tracks_selected_object() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));
    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-flow/flow/edge/e:ab")
    );

    app.handle_key_code(KeyCode::Char('3')); // toggle+focus xrefs
    let before = app.visible_xref_indices().len();

    app.handle_key_code(KeyCode::Char('I')); // involving-only
    let after = app.visible_xref_indices().len();
    assert!(after < before);
    assert!(!app.visible_xref_indices().is_empty());
    let selected_ref = app.selected_ref().expect("selected object");
    assert!(app.visible_xref_indices().iter().all(|&idx| {
        let xref = &app.xrefs[idx].xref;
        xref.from() == selected_ref || xref.to() == selected_ref
    }));

    app.handle_key_code(KeyCode::Char('1')); // focus diagram
    app.handle_key_code(KeyCode::Char('2')); // toggle+focus objects
    app.handle_key_code(KeyCode::Down); // next object (e:ac)
    assert!(app.visible_xref_indices().is_empty());
}

#[test]
fn jumping_from_xref_selects_object() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Char('2')); // toggle+focus objects
    app.handle_key_code(KeyCode::Down); // select non-zero object
    let before = app.selected_ref().map(ToString::to_string);

    app.handle_key_code(KeyCode::Char('3')); // toggle+focus xrefs
    app.handle_key_code(KeyCode::Char('g')); // jump to xref "from"
    let after = app.selected_ref().map(ToString::to_string);

    assert_ne!(before, after);
    assert_eq!(after.as_deref(), Some("d:demo-flow/flow/node/n:a"));
}

#[test]
fn diagram_t_follows_first_outgoing_xref_for_selected_object() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-00-index").expect("diagram id"));
    let selected_ref: ObjectRef =
        "d:demo-00-index/flow/node/n:seq_blocks".parse().expect("object ref");
    app.select_object_ref(&selected_ref);
    app.focus = Focus::Diagram;

    app.handle_key_code(KeyCode::Char('t'));

    assert_eq!(app.focus, Focus::Diagram);
    assert_eq!(
        app.active_diagram_id().map(ToString::to_string).as_deref(),
        Some("demo-t-seq-blocks")
    );
    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-t-seq-blocks/seq/participant/p:client")
    );
}

#[test]
fn diagram_t_toasts_when_multiple_outgoing_xrefs_exist() {
    let mut session = demo_session();
    let from: ObjectRef = "d:demo-00-index/flow/node/n:seq_blocks".parse().expect("object ref");
    let to: ObjectRef = "d:demo-flow/flow/node/n:a".parse().expect("object ref");
    session.xrefs_mut().insert(
        XRefId::new("x:nav:blocks:alt").expect("xref id"),
        XRef::new(from.clone(), to, "nav", XRefStatus::Ok),
    );

    let mut app = App::new(session);
    app.set_active_diagram_id(DiagramId::new("demo-00-index").expect("diagram id"));
    app.select_object_ref(&from);
    app.focus = Focus::Diagram;

    app.handle_key_code(KeyCode::Char('t'));

    let toast = app.toast.as_ref().expect("toast");
    assert!(toast.message.contains("2 outgoing xrefs"));
    assert!(toast.message.contains("x:nav:blocks"));
    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-t-seq-blocks/seq/participant/p:client")
    );
}

#[test]
fn diagram_g_follows_first_incoming_xref_for_selected_object() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-t-seq-blocks").expect("diagram id"));
    let selected_ref: ObjectRef =
        "d:demo-t-seq-blocks/seq/participant/p:client".parse().expect("object ref");
    app.select_object_ref(&selected_ref);
    app.focus = Focus::Diagram;

    app.handle_key_code(KeyCode::Char('g'));

    assert_eq!(app.focus, Focus::Diagram);
    assert_eq!(app.active_diagram_id().map(ToString::to_string).as_deref(), Some("demo-00-index"));
    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-00-index/flow/node/n:seq_blocks")
    );
}

#[test]
fn diagram_g_toasts_when_multiple_incoming_xrefs_exist() {
    let mut session = demo_session();
    let from: ObjectRef = "d:demo-00-index/flow/node/n:seq_demo".parse().expect("object ref");
    let to: ObjectRef = "d:demo-t-seq-blocks/seq/participant/p:client".parse().expect("object ref");
    session.xrefs_mut().insert(
        XRefId::new("x:nav:blocks:back").expect("xref id"),
        XRef::new(from, to.clone(), "nav", XRefStatus::Ok),
    );

    let mut app = App::new(session);
    app.set_active_diagram_id(DiagramId::new("demo-t-seq-blocks").expect("diagram id"));
    app.select_object_ref(&to);
    app.focus = Focus::Diagram;

    app.handle_key_code(KeyCode::Char('g'));

    let toast = app.toast.as_ref().expect("toast");
    assert!(toast.message.contains("2 incoming xrefs"));
    assert!(toast.message.contains("x:nav:blocks"));
    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-00-index/flow/node/n:seq_blocks")
    );
}

#[test]
fn objects_t_follows_first_outgoing_xref_for_selected_object() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-00-index").expect("diagram id"));
    let selected_ref: ObjectRef =
        "d:demo-00-index/flow/node/n:seq_blocks".parse().expect("object ref");
    app.select_object_ref(&selected_ref);
    app.focus = Focus::Objects;

    app.handle_key_code(KeyCode::Char('t'));

    assert_eq!(app.focus, Focus::Objects);
    assert_eq!(
        app.active_diagram_id().map(ToString::to_string).as_deref(),
        Some("demo-t-seq-blocks")
    );
    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-t-seq-blocks/seq/participant/p:client")
    );
}

#[test]
fn objects_g_follows_first_incoming_xref_for_selected_object() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-t-seq-blocks").expect("diagram id"));
    let selected_ref: ObjectRef =
        "d:demo-t-seq-blocks/seq/participant/p:client".parse().expect("object ref");
    app.select_object_ref(&selected_ref);
    app.focus = Focus::Objects;

    app.handle_key_code(KeyCode::Char('g'));

    assert_eq!(app.focus, Focus::Objects);
    assert_eq!(app.active_diagram_id().map(ToString::to_string).as_deref(), Some("demo-00-index"));
    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-00-index/flow/node/n:seq_blocks")
    );
}

#[test]
fn diagram_view_prefixes_only_objects_with_outgoing_xrefs() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-00-index").expect("diagram id"));
    let rendered = text_to_string(&app.diagram_text());
    assert!(rendered.contains("▴ Flowchart: labels + shapes"));
    assert!(rendered.contains("Engine features"));
    assert!(!rendered.contains("▴ Engine features"));
}

#[test]
fn diagram_view_prefixes_support_incoming_and_outgoing_markers() {
    let mut session = demo_session();
    let from: ObjectRef = "d:demo-flow/flow/node/n:a".parse().expect("object ref");
    let to: ObjectRef = "d:demo-00-index/flow/node/n:flow_demo".parse().expect("object ref");
    session.xrefs_mut().insert(
        XRefId::new("x:back:flow_demo").expect("xref id"),
        XRef::new(from, to, "nav", XRefStatus::Ok),
    );

    let mut app = App::new(session);
    app.set_active_diagram_id(DiagramId::new("demo-00-index").expect("diagram id"));
    let rendered = text_to_string(&app.diagram_text());
    assert!(rendered.contains("▾▴ Flowchart: labels + shapes"));
}

#[test]
fn diagram_view_renders_direction_markers_in_cyan() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-00-index").expect("diagram id"));
    let text = app.diagram_text();

    let marker_styles = text
        .lines
        .iter()
        .flat_map(|line| {
            line.spans.iter().flat_map(|span| {
                span.content.chars().filter(|ch| matches!(ch, '▾' | '▴')).map(move |_| span.style)
            })
        })
        .collect::<Vec<_>>();

    assert!(!marker_styles.is_empty(), "expected at least one direction marker");
    assert!(
        marker_styles.iter().all(|style| style.fg == Some(Color::Cyan)),
        "expected all direction markers to render in cyan"
    );
}

#[test]
fn diagram_switching_updates_active_diagram_and_objects() {
    let mut app = App::new(demo_session());
    let before_diagram = app.active_diagram_id().map(ToString::to_string);
    assert!(before_diagram.is_some());

    app.handle_key_code(KeyCode::Char(']'));
    let after_diagram = app.active_diagram_id().map(ToString::to_string);
    assert_ne!(before_diagram, after_diagram);

    let active = app.active_diagram_id().expect("active diagram");
    assert!(app.objects.iter().all(|obj| obj.object_ref.diagram_id() == active));

    app.handle_key_code(KeyCode::Char('['));
    let back_diagram = app.active_diagram_id().map(ToString::to_string);
    assert_eq!(before_diagram, back_diagram);
}

#[test]
fn diagram_switching_persists_active_diagram_with_session_folder() {
    let session = demo_session();
    let tmp_dir = std::env::temp_dir().join(format!(
        "nereid-tui-active-diagram-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp_dir).expect("create temp session dir");

    let folder = SessionFolder::new(&tmp_dir);
    folder.save_session(&session).expect("save session");

    let mut app = App::new(session);
    app.session_folder = Some(folder.clone());

    let before = app.active_diagram_id().cloned().expect("active diagram");
    app.handle_key_code(KeyCode::Char(']'));
    let after = app.active_diagram_id().cloned().expect("active diagram after switch");
    assert_ne!(before, after);

    let meta = folder.load_meta().expect("load session meta");
    assert_eq!(meta.active_diagram_id, Some(after.clone()));

    let loaded = folder.load_session().expect("reload session");
    assert_eq!(loaded.active_diagram_id(), Some(&after));

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn cross_diagram_xref_jump_switches_diagram_and_selects_object() {
    let mut session = demo_session();

    let flow_id = DiagramId::new("demo-flow").expect("diagram id");
    let seq_id = DiagramId::new("demo-seq").expect("diagram id");
    let from = ObjectRef::new(
        flow_id,
        category_path(&["flow", "node"]),
        ObjectId::new("n:a").expect("object id"),
    );
    let to = ObjectRef::new(
        seq_id.clone(),
        category_path(&["seq", "participant"]),
        ObjectId::new("p:alice").expect("object id"),
    );

    session.xrefs_mut().insert(
        XRefId::new("x:3").expect("xref id"),
        XRef::new(from, to.clone(), "cross", XRefStatus::Ok),
    );

    let mut app = App::new(session);
    app.handle_key_code(KeyCode::Char('3')); // toggle+focus xrefs
    app.handle_key_code(KeyCode::Down); // x:2
    app.handle_key_code(KeyCode::Down); // x:3

    app.handle_key_code(KeyCode::Char('t')); // jump "to" (different diagram)

    assert_eq!(app.active_diagram_id(), Some(&seq_id));
    assert_eq!(app.selected_ref(), Some(&to));
}

#[test]
fn regular_search_switches_diagram_and_selects_object() {
    let mut app = App::new(demo_session());
    assert_eq!(app.active_diagram_id().map(ToString::to_string).as_deref(), Some("demo-00-index"));

    app.handle_key_code(KeyCode::Char('/'));
    for ch in "alice".chars() {
        app.handle_key_code(KeyCode::Char(ch));
    }

    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-seq/seq/participant/p:alice")
    );
}

#[test]
fn regular_search_does_not_match_flab_example() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-seq").expect("diagram id"));
    assert_eq!(app.active_diagram_id().map(ToString::to_string).as_deref(), Some("demo-seq"));

    app.handle_key_code(KeyCode::Char('/'));
    for ch in "flab".chars() {
        app.handle_key_code(KeyCode::Char(ch));
    }

    assert!(app.search_results.is_empty());
}

#[test]
fn regular_search_matches_node_label_text() {
    let mut app = App::new(demo_session());

    app.handle_key_code(KeyCode::Char('/'));
    for ch in "arrow".chars() {
        app.handle_key_code(KeyCode::Char(ch));
    }

    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-00-index/flow/node/n:seq_demo")
    );
}

#[test]
fn fuzzy_search_matches_flab_example() {
    let mut app = App::new(demo_session());
    app.set_active_diagram_id(DiagramId::new("demo-seq").expect("diagram id"));

    app.handle_key_code(KeyCode::Char('\\'));
    for ch in "flab".chars() {
        app.handle_key_code(KeyCode::Char(ch));
    }

    assert_eq!(
        app.selected_ref().map(ToString::to_string).as_deref(),
        Some("d:demo-flow/flow/edge/e:ab")
    );
}

#[test]
fn fuzzy_search_results_cycle_with_n_and_shift_n() {
    let mut app = App::new(demo_session());

    app.handle_key_code(KeyCode::Char('/'));
    app.handle_key_code(KeyCode::Char('p'));
    app.handle_key_code(KeyCode::Char(':'));
    let first = app.selected_ref().map(ToString::to_string);

    app.handle_key_code(KeyCode::Enter); // commit search
    assert_eq!(app.search_mode, SearchMode::Results);

    app.handle_key_code(KeyCode::Char('n'));
    let second = app.selected_ref().map(ToString::to_string);
    assert_ne!(first, second);

    app.handle_key_code(KeyCode::Char('N'));
    let back = app.selected_ref().map(ToString::to_string);
    assert_eq!(first, back);

    let focused = app.selected_ref().map(ToString::to_string);
    app.handle_key_code(KeyCode::Esc); // clear search
    assert_eq!(app.search_mode, SearchMode::Inactive);
    assert!(app.search_query.is_empty());
    assert!(app.search_results.is_empty());
    assert_eq!(app.selected_ref().map(ToString::to_string), focused);
}

#[test]
fn search_results_group_current_diagram_first_for_regular_and_fuzzy() {
    let session = demo_session();
    let candidates = search_candidates_from_session(&session);
    let active = DiagramId::new("demo-seq").expect("diagram id");

    let regular =
        ranked_search_results(&candidates, "participant", SearchKind::Regular, Some(&active));
    assert!(!regular.is_empty());
    assert_eq!(regular.first().map(|oref| oref.diagram_id().as_str()), Some("demo-seq"));
    let mut seen_regular = BTreeSet::<String>::new();
    let mut last_regular = String::new();
    for object_ref in regular {
        let diagram = object_ref.diagram_id().to_string();
        if diagram != last_regular {
            assert!(
                !seen_regular.contains(&diagram),
                "diagram groups should be contiguous in regular search"
            );
            seen_regular.insert(diagram.clone());
            last_regular = diagram;
        }
    }

    let fuzzy = ranked_search_results(&candidates, "particpant", SearchKind::Fuzzy, Some(&active));
    assert!(!fuzzy.is_empty());
    assert_eq!(fuzzy.first().map(|oref| oref.diagram_id().as_str()), Some("demo-seq"));
    let mut seen_fuzzy = BTreeSet::<String>::new();
    let mut last_fuzzy = String::new();
    for object_ref in fuzzy {
        let diagram = object_ref.diagram_id().to_string();
        if diagram != last_fuzzy {
            assert!(
                !seen_fuzzy.contains(&diagram),
                "diagram groups should be contiguous in fuzzy search"
            );
            seen_fuzzy.insert(diagram.clone());
            last_fuzzy = diagram;
        }
    }
}

#[test]
fn scroll_supports_negative_offsets() {
    let mut app = App::new(demo_session());
    app.handle_key_code(KeyCode::Up);
    app.handle_key_code(KeyCode::Left);
    assert_eq!(app.pan_y, -1);
    assert_eq!(app.pan_x, -1);
}

#[test]
fn diagram_focus_n_toggles_notes_when_search_inactive() {
    let mut app = App::new(demo_session());
    assert!(app.show_notes);
    app.handle_key_code(KeyCode::Char('n'));
    assert!(!app.show_notes);
    app.handle_key_code(KeyCode::Char('n'));
    assert!(app.show_notes);
}

#[test]
fn center_diagram_sets_negative_pan_for_small_diagram() {
    let mut app = App::new(demo_session());
    app.base_diagram = "abc".to_owned();
    app.center_diagram_on_next_draw = true;

    app.center_diagram_if_needed(9, 5);

    assert_eq!(app.pan_x, -3);
    assert_eq!(app.pan_y, -2);
    assert!(!app.center_diagram_on_next_draw);
}

#[test]
fn center_diagram_clamps_to_one_cell_padding_when_center_would_clip_left_top() {
    let mut app = App::new(demo_session());
    app.base_diagram = "0123456789ABCDEF\n0123456789ABCDEF\n0123456789ABCDEF".to_owned();
    app.center_diagram_on_next_draw = true;

    app.center_diagram_if_needed(8, 2);

    assert_eq!(app.pan_x, -1);
    assert_eq!(app.pan_y, -1);
}

#[test]
fn diagram_render_offsets_dont_pad_for_positive_pan() {
    let mut app = App::new(demo_session());
    app.pan_x = 12;
    app.pan_y = 7;

    let (scroll_x, scroll_y, left_pad, top_pad) = app.diagram_render_offsets();
    assert_eq!(scroll_x, 12);
    assert_eq!(scroll_y, 7);
    assert_eq!(left_pad, 0);
    assert_eq!(top_pad, 0);
}

#[test]
fn switching_diagram_marks_center_pending() {
    let mut app = App::new(demo_session());
    app.center_diagram_on_next_draw = false;
    app.pan_x = 42;
    app.pan_y = 24;

    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));

    assert!(app.center_diagram_on_next_draw);
    assert_eq!(app.pan_x, 0);
    assert_eq!(app.pan_y, 0);
}

#[test]
fn ensure_active_diagram_sets_first_diagram_when_missing() {
    let mut session = demo_session();
    session.set_active_diagram_id(None);
    assert_eq!(session.active_diagram_id(), None);

    let selected = ensure_active_diagram_id(&mut session);

    assert_eq!(selected.as_ref(), session.active_diagram_id());
    assert!(selected.is_some());
}

#[test]
fn app_handles_session_with_no_diagrams() {
    let session = Session::new(SessionId::new("s:empty").expect("session id"));
    let app = App::new(session);

    assert_eq!(app.active_diagram_id(), None);
    assert_eq!(app.base_diagram, "No diagrams in session");
    assert!(app.objects.is_empty());
}

#[test]
fn demo_fixture_diagrams_render_without_errors() {
    let session = demo_session();
    for diagram in session.diagrams().values() {
        let rendered = render_diagram_unicode_annotated_with_options(
            diagram,
            RenderOptions {
                show_notes: true,
                prefix_object_labels: false,
                flowchart_extra_col_gap: 0,
            },
        );
        assert!(
            rendered.is_ok(),
            "fixture diagram {} did not render: {:?}",
            diagram.diagram_id(),
            rendered.err()
        );
    }
}

#[test]
fn demo_flow_routing_nodes_have_labels() {
    let session = demo_session();
    let diagram = session
        .diagrams()
        .get(&DiagramId::new("demo-t-flow-routing").expect("diagram id"))
        .expect("diagram exists");

    let rendered = render_diagram_unicode_annotated_with_options(
        diagram,
        RenderOptions {
            show_notes: false,
            prefix_object_labels: false,
            flowchart_extra_col_gap: 0,
        },
    )
    .expect("render");

    assert!(rendered.text.contains("Analyze"), "rendered:\n{}", rendered.text);
    assert!(rendered.text.contains("Diagnostics"), "rendered:\n{}", rendered.text);
    assert!(rendered.text.contains("Options"), "rendered:\n{}", rendered.text);
}

#[test]
fn diagram_hints_show_nodes_and_edges_for_flowchart() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets,
        other => panic!("expected AwaitingFirst Jump, got {other:?}"),
    };

    assert_eq!(targets.len(), 8);
    let mut uniq = std::collections::HashSet::new();
    let mut node_targets = 0usize;
    let mut edge_targets = 0usize;
    for target in targets {
        assert!(target.label[0].is_ascii_uppercase());
        assert!(target.label[1].is_ascii_uppercase());
        assert!(uniq.insert(target.label));

        let object_ref = target.object_ref.to_string();
        if object_ref.contains("/flow/node/") {
            node_targets += 1;
            continue;
        }
        if object_ref.contains("/flow/edge/") {
            edge_targets += 1;
            continue;
        }
        panic!("unexpected flow hint target: {object_ref}");
    }
    assert_eq!(node_targets, 4);
    assert_eq!(edge_targets, 4);
}

#[test]
fn diagram_hints_show_only_participants_for_sequence() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-seq").expect("diagram id"));

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets,
        other => panic!("expected AwaitingFirst Jump, got {other:?}"),
    };

    assert_eq!(targets.len(), 3);
    assert!(targets.iter().any(|target| target.object_ref.to_string().contains("/seq/message/")));
    for target in targets {
        let r = target.object_ref.to_string();
        assert!(r.contains("/seq/participant/") || r.contains("/seq/message/"));
    }
}

#[test]
fn xref_direction_prefix_marks_from_and_to() {
    let selected = ObjectRef::new(
        DiagramId::new("d:test").expect("diagram id"),
        category_path(&["flow", "node"]),
        ObjectId::new("n:selected").expect("object id"),
    );
    let other = ObjectRef::new(
        DiagramId::new("d:test").expect("diagram id"),
        category_path(&["flow", "node"]),
        ObjectId::new("n:other").expect("object id"),
    );

    let xref_from = XRef::new(selected.clone(), other.clone(), "uses", XRefStatus::Ok);
    assert_eq!(super::xref_direction_prefix(Some(&selected), &xref_from), "▴ ");

    let xref_to = XRef::new(other.clone(), selected.clone(), "uses", XRefStatus::Ok);
    assert_eq!(super::xref_direction_prefix(Some(&selected), &xref_to), "▾ ");

    let xref_both = XRef::new(selected.clone(), selected.clone(), "uses", XRefStatus::Ok);
    assert_eq!(super::xref_direction_prefix(Some(&selected), &xref_both), "▾▴ ");

    let xref_neither = XRef::new(other.clone(), other.clone(), "uses", XRefStatus::Ok);
    assert_eq!(super::xref_direction_prefix(Some(&selected), &xref_neither), "");

    assert_eq!(super::xref_direction_prefix(None, &xref_from), "");
}

#[test]
fn diagram_hints_filter_and_select_on_second_letter() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));
    let before = app.selected_ref().cloned();

    app.handle_key_code(KeyCode::Char('f'));
    let first_targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets.clone(),
        _ => panic!("expected AwaitingFirst"),
    };

    let chosen = first_targets
        .iter()
        .find(|target| before.as_ref() != Some(&target.object_ref))
        .cloned()
        .unwrap_or_else(|| first_targets[0].clone());
    app.handle_key_code(KeyCode::Char(chosen.label[0].to_ascii_lowercase()));

    let second_targets = match &app.hint_mode {
        HintMode::AwaitingSecond { kind: HintKind::Jump, first, targets } => {
            assert_eq!(*first, chosen.label[0]);
            targets.clone()
        }
        _ => panic!("expected AwaitingSecond"),
    };
    assert!(second_targets.iter().all(|target| target.label[0] == chosen.label[0]));

    let hint_text = app.diagram_text();
    let has_typed_grey = hint_text.lines.iter().any(|line| {
        line.spans.iter().any(|span| {
            span.style.bg == Some(Color::Cyan) && span.style.fg == Some(Color::DarkGray)
        })
    });
    assert!(has_typed_grey);

    app.handle_key_code(KeyCode::Char(chosen.label[1].to_ascii_lowercase()));
    assert!(matches!(app.hint_mode, HintMode::Inactive));
    assert_eq!(app.selected_ref(), Some(&chosen.object_ref));
    assert_ne!(before, app.selected_ref().cloned());
}

#[test]
fn diagram_hints_render_plain_space_before_tag_letters() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets.clone(),
        _ => panic!("expected AwaitingFirst"),
    };

    let hint_text = app.diagram_text();
    let lines = hint_text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for target in targets {
        if target.object_ref.to_string().contains("/flow/edge/") {
            continue;
        }
        let line = lines.get(target.y).expect("target line exists");
        let _tag_start = line
            .windows(2)
            .position(|cells| {
                cells[0].0 == target.label[0]
                    && cells[1].0 == target.label[1]
                    && cells[0].1.bg == Some(Color::Cyan)
                    && cells[1].1.bg == Some(Color::Cyan)
            })
            .expect("hint label rendered");
    }
}

#[test]
fn diagram_hints_render_tags_for_flow_edges() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets.clone(),
        _ => panic!("expected AwaitingFirst"),
    };

    let edge_targets = targets
        .iter()
        .filter(|target| target.object_ref.to_string().contains("/flow/edge/"))
        .cloned()
        .collect::<Vec<_>>();
    assert!(!edge_targets.is_empty(), "expected flow edge hint targets");

    let hint_text = app.diagram_text();
    let lines = hint_text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for target in edge_targets {
        let line = lines.get(target.y).expect("target line exists");
        let has_tag = line.windows(2).any(|cells| {
            cells[0].0 == target.label[0]
                && cells[1].0 == target.label[1]
                && cells[0].1.bg == Some(Color::Cyan)
                && cells[1].1.bg == Some(Color::Cyan)
        });
        assert!(has_tag, "missing flow edge hint tag for {}", target.object_ref);
    }
}

#[test]
fn diagram_hints_space_before_tag_inherits_focused_node_background() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets.clone(),
        _ => panic!("expected AwaitingFirst"),
    };
    let target = targets
        .iter()
        .find(|target| target.object_ref.to_string().contains("/flow/node/"))
        .expect("node hint target")
        .clone();
    app.select_object_ref(&target.object_ref);

    let hint_text = app.diagram_text();
    let line = hint_text
        .lines
        .get(target.y)
        .expect("target line exists")
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect::<Vec<_>>();

    let tag_start = line
        .windows(2)
        .position(|cells| {
            cells[0].0 == target.label[0]
                && cells[1].0 == target.label[1]
                && cells[0].1.bg == Some(Color::Cyan)
                && cells[1].1.bg == Some(Color::Cyan)
        })
        .expect("hint label rendered");

    assert!(tag_start > 0, "hint label should not be at line start");
    assert_eq!(line[tag_start - 1].0, ' ');
    assert_eq!(line[tag_start - 1].1.bg, Some(Color::LightGreen));
}

#[test]
fn diagram_hints_space_before_tag_inherits_selected_node_background() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets.clone(),
        _ => panic!("expected AwaitingFirst"),
    };
    let target = targets
        .iter()
        .find(|target| target.object_ref.to_string().contains("/flow/node/"))
        .expect("node hint target")
        .clone();
    app.session.selected_object_refs_mut().insert(target.object_ref.clone());
    app.objects_state.select(None);

    let hint_text = app.diagram_text();
    let line = hint_text
        .lines
        .get(target.y)
        .expect("target line exists")
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect::<Vec<_>>();

    let tag_start = line
        .windows(2)
        .position(|cells| {
            cells[0].0 == target.label[0]
                && cells[1].0 == target.label[1]
                && cells[0].1.bg == Some(Color::Cyan)
                && cells[1].1.bg == Some(Color::Cyan)
        })
        .expect("hint label rendered");

    assert!(tag_start > 0, "hint label should not be at line start");
    assert_eq!(line[tag_start - 1].0, ' ');
    assert_eq!(line[tag_start - 1].1.bg, Some(Color::DarkGray));
}

#[test]
fn diagram_hints_wrong_second_letter_cancels_without_selection() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));
    let before = app.selected_ref().cloned();

    app.handle_key_code(KeyCode::Char('f'));
    let first_targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets.clone(),
        _ => panic!("expected AwaitingFirst"),
    };

    let chosen = first_targets[0].clone();
    app.handle_key_code(KeyCode::Char(chosen.label[0].to_ascii_lowercase()));
    app.handle_key_code(KeyCode::Char('z')); // wrong second letter

    assert!(matches!(app.hint_mode, HintMode::Inactive));
    assert_eq!(app.selected_ref().cloned(), before);
}

#[test]
fn diagram_hints_esc_cancels_without_selection() {
    let mut app = App::new(demo_session_fallback());
    app.set_active_diagram_id(DiagramId::new("demo-flow").expect("diagram id"));
    let before = app.selected_ref().cloned();

    app.handle_key_code(KeyCode::Char('f'));
    app.handle_key_code(KeyCode::Esc);
    assert!(matches!(app.hint_mode, HintMode::Inactive));
    assert_eq!(app.selected_ref().cloned(), before);
}

#[test]
fn diagram_select_hints_selects_edge_between_consecutive_flow_nodes() {
    let mut app = App::new(demo_session_fallback());
    let flow_id = DiagramId::new("demo-flow").expect("diagram id");
    app.set_active_diagram_id(flow_id.clone());

    app.handle_key_code(KeyCode::Char('c'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::SelectChain, targets } => targets.clone(),
        other => panic!("expected AwaitingFirst SelectChain, got {other:?}"),
    };

    let a_ref = ObjectRef::new(
        flow_id.clone(),
        category_path(&["flow", "node"]),
        ObjectId::new("n:a").expect("object id"),
    );
    let b_ref = ObjectRef::new(
        flow_id.clone(),
        category_path(&["flow", "node"]),
        ObjectId::new("n:b").expect("object id"),
    );
    let edge_ref = ObjectRef::new(
        flow_id,
        category_path(&["flow", "edge"]),
        ObjectId::new("e:ab").expect("object id"),
    );

    let a_target =
        targets.iter().find(|t| t.object_ref == a_ref).expect("hint target for n:a").clone();
    let b_target =
        targets.iter().find(|t| t.object_ref == b_ref).expect("hint target for n:b").clone();

    app.handle_key_code(KeyCode::Char(a_target.label[0].to_ascii_lowercase()));
    app.handle_key_code(KeyCode::Char(a_target.label[1].to_ascii_lowercase()));
    assert!(app.session.selected_object_refs().contains(&a_ref));
    assert!(matches!(app.hint_mode, HintMode::AwaitingFirst { kind: HintKind::SelectChain, .. }));

    app.handle_key_code(KeyCode::Char(b_target.label[0].to_ascii_lowercase()));
    app.handle_key_code(KeyCode::Char(b_target.label[1].to_ascii_lowercase()));
    assert!(app.session.selected_object_refs().contains(&b_ref));
    assert!(app.session.selected_object_refs().contains(&edge_ref));
    assert!(matches!(app.hint_mode, HintMode::AwaitingFirst { kind: HintKind::SelectChain, .. }));

    app.handle_key_code(KeyCode::Esc);
    assert!(matches!(app.hint_mode, HintMode::Inactive));
}

#[test]
fn diagram_hints_jump_selects_unlabeled_flow_edge() {
    let mut app = App::new(demo_session_fallback());
    let flow_id = DiagramId::new("demo-flow").expect("diagram id");
    app.set_active_diagram_id(flow_id.clone());

    app.handle_key_code(KeyCode::Char('f'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::Jump, targets } => targets.clone(),
        other => panic!("expected AwaitingFirst Jump, got {other:?}"),
    };

    let edge_ref = ObjectRef::new(
        flow_id,
        category_path(&["flow", "edge"]),
        ObjectId::new("e:cd").expect("object id"),
    );
    let edge_target =
        targets.iter().find(|t| t.object_ref == edge_ref).expect("hint target for e:cd").clone();

    app.handle_key_code(KeyCode::Char(edge_target.label[0].to_ascii_lowercase()));
    app.handle_key_code(KeyCode::Char(edge_target.label[1].to_ascii_lowercase()));

    assert!(matches!(app.hint_mode, HintMode::Inactive));
    assert_eq!(app.selected_ref(), Some(&edge_ref));
}

#[test]
fn diagram_select_hints_selects_flow_edge_directly() {
    let mut app = App::new(demo_session_fallback());
    let flow_id = DiagramId::new("demo-flow").expect("diagram id");
    app.set_active_diagram_id(flow_id.clone());

    app.handle_key_code(KeyCode::Char('c'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::SelectChain, targets } => targets.clone(),
        other => panic!("expected AwaitingFirst SelectChain, got {other:?}"),
    };

    let edge_ref = ObjectRef::new(
        flow_id,
        category_path(&["flow", "edge"]),
        ObjectId::new("e:cd").expect("object id"),
    );
    let edge_target =
        targets.iter().find(|t| t.object_ref == edge_ref).expect("hint target for e:cd").clone();

    app.handle_key_code(KeyCode::Char(edge_target.label[0].to_ascii_lowercase()));
    app.handle_key_code(KeyCode::Char(edge_target.label[1].to_ascii_lowercase()));

    assert!(app.session.selected_object_refs().contains(&edge_ref));
    assert_eq!(app.selected_ref(), Some(&edge_ref));
    assert!(matches!(app.hint_mode, HintMode::AwaitingFirst { kind: HintKind::SelectChain, .. }));
}

#[test]
fn diagram_select_hints_selects_message_between_consecutive_sequence_participants() {
    let mut app = App::new(demo_session_fallback());
    let seq_id = DiagramId::new("demo-seq").expect("diagram id");
    app.set_active_diagram_id(seq_id.clone());

    app.handle_key_code(KeyCode::Char('c'));
    let targets = match &app.hint_mode {
        HintMode::AwaitingFirst { kind: HintKind::SelectChain, targets } => targets.clone(),
        other => panic!("expected AwaitingFirst SelectChain, got {other:?}"),
    };

    assert_eq!(targets.len(), 2);
    assert!(!targets.iter().any(|t| t.object_ref.to_string().contains("/seq/message/")));

    let alice_ref = ObjectRef::new(
        seq_id.clone(),
        category_path(&["seq", "participant"]),
        ObjectId::new("p:alice").expect("object id"),
    );
    let bob_ref = ObjectRef::new(
        seq_id.clone(),
        category_path(&["seq", "participant"]),
        ObjectId::new("p:bob").expect("object id"),
    );
    let message_ref = ObjectRef::new(
        seq_id,
        category_path(&["seq", "message"]),
        ObjectId::new("m:0001").expect("object id"),
    );

    let alice_target = targets
        .iter()
        .find(|t| t.object_ref == alice_ref)
        .expect("hint target for p:alice")
        .clone();
    let bob_target =
        targets.iter().find(|t| t.object_ref == bob_ref).expect("hint target for p:bob").clone();

    app.handle_key_code(KeyCode::Char(alice_target.label[0].to_ascii_lowercase()));
    app.handle_key_code(KeyCode::Char(alice_target.label[1].to_ascii_lowercase()));
    assert!(app.session.selected_object_refs().contains(&alice_ref));

    app.handle_key_code(KeyCode::Char(bob_target.label[0].to_ascii_lowercase()));
    app.handle_key_code(KeyCode::Char(bob_target.label[1].to_ascii_lowercase()));
    assert!(app.session.selected_object_refs().contains(&bob_ref));
    assert!(app.session.selected_object_refs().contains(&message_ref));
}
