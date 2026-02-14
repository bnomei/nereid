// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

/// Layout, title, footer, help, and style helpers used by TUI rendering.
fn stack_main_panes_vertically(area: Rect, sidebar_panel_count: usize) -> bool {
    if sidebar_panel_count >= 3 {
        area.width < 110
    } else {
        area.width < 90
    }
}

fn footer_uses_compact_mode(area: Rect, sidebar_panel_count: usize) -> bool {
    stack_main_panes_vertically(area, sidebar_panel_count)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Diagram,
    Objects,
    XRefs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusOwner {
    Human,
    Agent,
}

impl Focus {
    fn cycle(self) -> Self {
        match self {
            Self::Diagram => Self::Objects,
            Self::Objects => Self::XRefs,
            Self::XRefs => Self::Diagram,
        }
    }

    fn cycle_back(self) -> Self {
        match self {
            Self::Diagram => Self::XRefs,
            Self::Objects => Self::Diagram,
            Self::XRefs => Self::Objects,
        }
    }
}

fn panel_border_style_for_focus(active: Focus, panel: Focus, owner: FocusOwner) -> Style {
    if active != panel {
        return Style::default();
    }

    Style::default().fg(focus_color_for_owner(owner))
}

fn focus_color_for_owner(owner: FocusOwner) -> Color {
    match owner {
        FocusOwner::Human => FOCUS_COLOR,
        FocusOwner::Agent => AGENT_FOCUS_COLOR,
    }
}

fn view_title(label: &str, key: char, tail: Option<&str>) -> String {
    let mut title = format!("─[{key}]─ {label}");
    if let Some(tail) = tail {
        let tail = tail.trim();
        if !tail.is_empty() {
            title.push(' ');
            title.push_str(tail);
        }
    }
    title.push(' ');
    title
}

fn diagram_view_title(
    diagram_id: &str,
    is_focused: bool,
    diagram_index: Option<usize>,
    diagram_total: usize,
) -> Line<'static> {
    let id_color = if is_focused {
        Color::White
    } else {
        Color::Gray
    };
    let counter = diagram_counter_label(diagram_index, diagram_total);
    Line::from(vec![
        Span::raw("─ Diagram ".to_owned()),
        Span::styled(counter, Style::default().fg(Color::LightGreen)),
        Span::raw(" ".to_owned()),
        Span::styled(diagram_id.to_owned(), Style::default().fg(id_color)),
        Span::raw(" ".to_owned()),
    ])
}

fn diagram_counter_label(diagram_index: Option<usize>, diagram_total: usize) -> String {
    if diagram_total == 0 {
        return "[0/0]".to_owned();
    }

    let width = diagram_total.to_string().len();
    let index = diagram_index.unwrap_or(0).min(diagram_total);
    format!("[{index:0width$}/{diagram_total}]")
}

fn clamp_positive_i32_to_u16(value: i32) -> u16 {
    value.max(0).min(u16::MAX as i32) as u16
}

fn pad_text(mut text: Text<'static>, left_pad: usize, top_pad: usize) -> Text<'static> {
    if left_pad == 0 && top_pad == 0 {
        return text;
    }

    if left_pad > 0 {
        let pad = " ".repeat(left_pad);
        for line in &mut text.lines {
            line.spans.insert(0, Span::raw(pad.clone()));
        }
    }

    if top_pad > 0 {
        let blank = Line::from(String::new());
        let mut lines = Vec::with_capacity(top_pad + text.lines.len());
        for _ in 0..top_pad {
            lines.push(blank.clone());
        }
        lines.extend(text.lines);
        text.lines = lines;
    }

    text
}

fn style_for_diagram_char(mut style: Style, ch: char) -> Style {
    if is_direction_marker(ch) {
        style.fg = Some(Color::Cyan);
    }
    style
}

fn is_direction_marker(ch: char) -> bool {
    matches!(ch, '▾' | '▴')
}

fn objects_item_bg(
    is_cursor: bool,
    is_selected: bool,
    objects_has_focus: bool,
    owner: FocusOwner,
) -> Option<Color> {
    if is_cursor {
        if is_selected && objects_has_focus {
            Some(Color::LightGreen)
        } else if is_selected {
            Some(Color::DarkGray)
        } else {
            Some(focus_color_for_owner(owner))
        }
    } else if is_selected {
        Some(Color::DarkGray)
    } else {
        None
    }
}

fn xref_item_style(status: XRefStatus, indirectly_selected: bool) -> Style {
    if indirectly_selected {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else {
        match status {
            XRefStatus::Ok => Style::default(),
            _ => Style::default().fg(Color::Red),
        }
    }
}

fn xrefs_cursor_highlight_style(focus: Focus, owner: FocusOwner) -> Style {
    if focus == Focus::XRefs {
        Style::default()
            .fg(Color::White)
            .bg(focus_color_for_owner(owner))
    } else {
        Style::default()
    }
}

fn xref_involves_selected(selected: Option<&ObjectRef>, xref: &XRef) -> bool {
    selected.is_some_and(|selected| xref.from() == selected || xref.to() == selected)
}

fn demo_palette_lines() -> Vec<Line<'static>> {
    let mut bg_spans = Vec::<Span<'static>>::new();
    let mut fg_spans = Vec::<Span<'static>>::new();

    for (idx, color) in demo_palette_colors().iter().copied().enumerate() {
        if idx > 0 {
            bg_spans.push(Span::raw(" "));
            fg_spans.push(Span::raw(" "));
        }

        bg_spans.push(Span::styled(
            format!("{idx:>2}"),
            Style::default()
                .fg(demo_palette_bg_text_color(idx))
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ));

        fg_spans.push(Span::styled(
            format!("{idx:>2}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }

    vec![Line::from(bg_spans), Line::from(fg_spans)]
}

fn demo_palette_bg_text_color(idx: usize) -> Color {
    match idx {
        0 | 1 | 4 | 5 | 8 => Color::White,
        _ => Color::Black,
    }
}

fn demo_palette_colors() -> [Color; 16] {
    [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ]
}

fn footer_help_line(app: &App, toast_suffix: &str, compact: bool) -> Line<'static> {
    let mut spans = Vec::<Span<'static>>::new();
    let follow_ai = if app.follow_ai { "a◼ " } else { "a◻ " };
    let diagram_hotkeys_disabled = app.follow_ai && app.focus == Focus::Diagram;

    if compact {
        let compact_hint = match app.hint_mode {
            HintMode::Inactive => "f",
            HintMode::AwaitingFirst { .. } | HintMode::AwaitingSecond { .. } => "2 letters",
        };
        push_footer_entry_with_separator(&mut spans, "AI", follow_ai, " | ");
        push_footer_entry_with_separator_maybe_disabled(
            &mut spans,
            "HINT",
            compact_hint,
            " | ",
            diagram_hotkeys_disabled,
        );
        push_footer_entry_with_separator(&mut spans, "HELP", "?", " | ");
        push_footer_entry_with_separator(&mut spans, "QUIT", "q", " | ");
    } else {
        match app.focus {
            Focus::Diagram => match app.hint_mode {
                HintMode::Inactive => {
                    let notes = if app.show_notes { "n◼ " } else { "n◻ " };
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "DIAGRAM",
                        "[]",
                        diagram_hotkeys_disabled,
                    );
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "HINT",
                        "f",
                        diagram_hotkeys_disabled,
                    );
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "CHAIN",
                        "c",
                        diagram_hotkeys_disabled,
                    );
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "EDIT",
                        "e",
                        diagram_hotkeys_disabled,
                    );
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "SELECT",
                        "⏡",
                        diagram_hotkeys_disabled,
                    );
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "YANK",
                        "y",
                        diagram_hotkeys_disabled,
                    );
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "XREF",
                        "g/t",
                        diagram_hotkeys_disabled,
                    );
                    push_footer_entry_maybe_disabled(
                        &mut spans,
                        "NOTES",
                        notes,
                        diagram_hotkeys_disabled,
                    );
                }
                HintMode::AwaitingFirst { kind, .. } | HintMode::AwaitingSecond { kind, .. } => {
                    match kind {
                        HintKind::Jump => {
                            push_footer_entry_maybe_disabled(
                                &mut spans,
                                "HINT",
                                "2 letters",
                                diagram_hotkeys_disabled,
                            );
                            push_footer_entry_maybe_disabled(
                                &mut spans,
                                "CANCEL",
                                "Esc",
                                diagram_hotkeys_disabled,
                            );
                        }
                        HintKind::SelectChain => {
                            push_footer_entry_maybe_disabled(
                                &mut spans,
                                "CHAIN",
                                "2 letters",
                                diagram_hotkeys_disabled,
                            );
                            push_footer_entry_maybe_disabled(
                                &mut spans,
                                "DONE",
                                "Esc",
                                diagram_hotkeys_disabled,
                            );
                        }
                    }
                }
            },
            Focus::Objects => {
                push_footer_entry(&mut spans, "SELECT", "⏡");
                push_footer_entry(&mut spans, "FILTER", "-");
                push_footer_entry(&mut spans, "HINT", "f");
                push_footer_entry(&mut spans, "CHAIN", "c");
                push_footer_entry(&mut spans, "YANK", "y");
                push_footer_entry(&mut spans, "JUMP", "g/t");
                push_footer_entry(&mut spans, "DIAGRAM", "[]");
            }
            Focus::XRefs => {
                push_footer_entry(&mut spans, "FILTER", "-/I");
                push_footer_entry(&mut spans, "JUMP", "g/t");
                push_footer_entry(&mut spans, "DIAGRAM", "[]");
            }
        }

        push_footer_entry(&mut spans, "AI", follow_ai);
        push_footer_entry(&mut spans, "HELP", "?");
        push_footer_entry(&mut spans, "QUIT", "q");
    }

    let toast_message = toast_suffix
        .strip_prefix(" | ")
        .unwrap_or(toast_suffix)
        .trim();
    if !toast_message.is_empty() {
        spans.push(Span::styled(" | ", Style::default().fg(FOOTER_LABEL_COLOR)));
        spans.push(Span::styled(
            "Toast:".to_owned(),
            Style::default().fg(FOOTER_LABEL_COLOR),
        ));
        spans.push(Span::raw(toast_message.to_owned()));
    }

    Line::from(spans)
}

fn search_footer_line(app: &App, toast_suffix: &str) -> Line<'static> {
    let query = app.search_query.as_str();
    let search_prefix = app.search_prefix();
    let (idx, total) = match app.search_results.len() {
        0 => (0usize, 0usize),
        n => (app.search_result_index.saturating_add(1), n),
    };

    let count = if query.is_empty() {
        None
    } else if total == 0 {
        Some("0".to_owned())
    } else {
        Some(format!("{idx}/{total}"))
    };

    let mut spans = vec![
        Span::styled(
            search_prefix.to_string(),
            Style::default()
                .fg(FOOTER_KEY_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(query.to_owned()),
        Span::raw("   "),
    ];
    if let Some(count) = count {
        spans.push(Span::styled(count, Style::default().fg(Color::LightGreen)));
    }

    if app.search_mode == SearchMode::Results {
        push_footer_entry_with_separator(&mut spans, "Next", "n/N", " | ");
    }
    push_footer_entry_with_separator(&mut spans, "Accept", "Enter", " | ");
    push_footer_entry_with_separator(&mut spans, "Close", "Esc", " | ");

    let toast_message = toast_suffix
        .strip_prefix(" | ")
        .unwrap_or(toast_suffix)
        .trim();
    if !toast_message.is_empty() {
        spans.push(Span::styled(" | ", Style::default().fg(FOOTER_LABEL_COLOR)));
        spans.push(Span::styled(
            "Toast:".to_owned(),
            Style::default().fg(FOOTER_LABEL_COLOR),
        ));
        spans.push(Span::raw(toast_message.to_owned()));
    }

    Line::from(spans)
}

fn footer_brand_line() -> Line<'static> {
    Line::from(vec![Span::styled(
        FOOTER_BRAND.to_owned(),
        Style::default().fg(FOOTER_BRAND_COLOR),
    )])
}

fn help_key_style() -> Style {
    Style::default()
        .fg(FOOTER_KEY_COLOR)
        .add_modifier(Modifier::BOLD)
}

fn help_header_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical_margin = (100u16.saturating_sub(height_percent)) / 2;
    let horizontal_margin = (100u16.saturating_sub(width_percent)) / 2;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(vertical_margin),
            Constraint::Percentage(height_percent),
            Constraint::Percentage(vertical_margin),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(horizontal_margin),
            Constraint::Percentage(width_percent),
            Constraint::Percentage(horizontal_margin),
        ])
        .split(vertical[1])[1]
}

fn help_kv(key: &str, desc: &str, key_width: usize, key_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:>width$}", width = key_width), key_style),
        Span::raw("  "),
        Span::raw(desc.to_owned()),
    ])
}

fn render_help(frame: &mut Frame<'_>, app: &mut App, main_area: Rect) {
    let area = centered_rect(82, 84, main_area);
    frame.render_widget(Clear, area);

    let key_style = help_key_style();
    let header_style = help_header_style();
    let dim_style = Style::default().fg(Color::DarkGray);

    let key_col_width = [
        "Tab/Shift-Tab",
        "j/k, ↑/↓, PgUp/PgDn, Home/End",
        "n/N",
        "Enter/Backspace",
    ]
    .iter()
    .map(|s| s.len())
    .max()
    .unwrap_or(0);

    let mut lines = Vec::<Line<'static>>::new();

    lines.push(Line::from(Span::styled("--- Global ---", header_style)));
    lines.push(help_kv("?", "Help (toggle)", key_col_width, key_style));
    lines.push(help_kv("q", "Quit", key_col_width, key_style));
    lines.push(help_kv("1", "Focus Diagram", key_col_width, key_style));
    lines.push(help_kv(
        "2/3",
        "Toggle+focus Objects/XRefs",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "4",
        "Toggle inspector panel",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "a",
        "Toggle follow AI highlight",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "d",
        "Deselect all in current diagram",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Tab/Shift-Tab",
        "Focus next/previous panel",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "[/]",
        "Previous/next diagram",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv("/", "Regular search", key_col_width, key_style));
    lines.push(help_kv("\\", "Fuzzy search", key_col_width, key_style));
    lines.push(help_kv(
        "n/N",
        "Search next/previous result",
        key_col_width,
        key_style,
    ));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("--- Search ---", header_style)));
    lines.push(help_kv("Type", "Update query", key_col_width, key_style));
    lines.push(help_kv(
        "Enter",
        "Commit results mode",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Backspace",
        "Delete query char",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv("Esc", "Clear search", key_col_width, key_style));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("--- Diagram ---", header_style)));
    lines.push(help_kv(
        "↑↓←→ / h/j/k/l",
        "Pan diagram by 1",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "H/J/K/L",
        "Pan diagram by 10",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "PgUp/PgDn",
        "Pan by page",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Home",
        "Reset pan to origin",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "n",
        "Toggle notes (when not searching)",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv("f", "Hint jump mode", key_col_width, key_style));
    lines.push(help_kv("c", "Chain hint mode", key_col_width, key_style));
    lines.push(help_kv(
        "e",
        "Edit active diagram in $EDITOR",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Space",
        "Toggle selected object",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "y",
        "Yank selected object ref",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "g/t",
        "Jump inbound/outbound",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Hint: 2 letters",
        "Choose hint target",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Hint Esc",
        "Cancel hint mode",
        key_col_width,
        key_style,
    ));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("--- Objects ---", header_style)));
    lines.push(help_kv(
        "↑/↓ or j/k",
        "Move object cursor",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Home/End",
        "First/last object",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Space",
        "Toggle selected object",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "-",
        "Filter selected-only",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv("f", "Hint jump mode", key_col_width, key_style));
    lines.push(help_kv("c", "Chain hint mode", key_col_width, key_style));
    lines.push(help_kv(
        "y",
        "Yank selected object ref",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "g/t",
        "Jump inbound/outbound",
        key_col_width,
        key_style,
    ));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("--- XRefs ---", header_style)));
    lines.push(help_kv(
        "↑/↓ or j/k",
        "Move xref cursor",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "Home/End",
        "First/last xref",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "-",
        "Toggle dangling-only filter",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "I",
        "Toggle involving-selection filter",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv(
        "g/t",
        "Jump to from/to endpoint",
        key_col_width,
        key_style,
    ));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("--- Help ---", header_style)));
    lines.push(help_kv(
        "j/k, ↑/↓, PgUp/PgDn, Home/End",
        "Scroll help",
        key_col_width,
        key_style,
    ));
    lines.push(help_kv("Esc/?", "Close help", key_col_width, key_style));
    lines.push(Line::from(vec![
        Span::styled("Note: ", dim_style),
        Span::styled("g/t", key_style),
        Span::styled(
            " follow the first matching xref (toast if multiple).",
            dim_style,
        ),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .title("─ Help ─")
        .border_style(Style::default().fg(FOCUS_COLOR))
        .title_style(
            Style::default()
                .fg(FOCUS_COLOR)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    app.help_viewport_height = inner.height;
    let max_scroll = lines
        .len()
        .saturating_sub(inner.height.max(1) as usize)
        .min(u16::MAX as usize) as u16;
    app.help_scroll = app.help_scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .style(Style::default())
        .wrap(Wrap { trim: false })
        .scroll((app.help_scroll, 0));
    frame.render_widget(paragraph, area);
}

fn push_footer_entry(spans: &mut Vec<Span<'static>>, label: &str, value: &str) {
    push_footer_entry_maybe_disabled(spans, label, value, false);
}

fn push_footer_entry_maybe_disabled(
    spans: &mut Vec<Span<'static>>,
    label: &str,
    value: &str,
    disabled: bool,
) {
    push_footer_entry_with_separator_maybe_disabled(spans, label, value, " | ", disabled);
}

fn push_footer_entry_with_separator(
    spans: &mut Vec<Span<'static>>,
    label: &str,
    value: &str,
    separator: &'static str,
) {
    push_footer_entry_with_separator_maybe_disabled(spans, label, value, separator, false);
}

fn push_footer_entry_with_separator_maybe_disabled(
    spans: &mut Vec<Span<'static>>,
    label: &str,
    value: &str,
    separator: &'static str,
    disabled: bool,
) {
    if !spans.is_empty() {
        spans.push(Span::styled(
            separator.to_owned(),
            Style::default().fg(FOOTER_LABEL_COLOR),
        ));
    }
    spans.push(Span::styled(
        format!("{}:", footer_label_ucfirst(label)),
        Style::default().fg(FOOTER_LABEL_COLOR),
    ));
    spans.extend(footer_value_spans(value, disabled));
}

fn footer_label_ucfirst(label: &str) -> String {
    let lower = label.to_lowercase();
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = first.to_uppercase().collect::<String>();
    out.push_str(chars.as_str());
    out
}

fn footer_value_spans(value: &str, disabled: bool) -> Vec<Span<'static>> {
    let color = if disabled {
        Color::DarkGray
    } else {
        FOOTER_KEY_COLOR
    };
    vec![Span::styled(
        value.to_owned(),
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD),
    )]
}
