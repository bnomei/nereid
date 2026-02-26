// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Terminal UI.
//!
//! Provides the interactive TUI shell (ratatui + crossterm), including a built-in demo session.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs, io,
    path::Path,
    process::Command,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use tokio::sync::Mutex;

use crate::format::mermaid::{
    export_flowchart, export_sequence_diagram, parse_flowchart, parse_sequence_diagram,
};
use crate::model::{
    CategoryPath, Diagram, DiagramAst, DiagramId, DiagramKind, FlowchartAst, ObjectId, ObjectRef,
    SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant, Session, SessionId,
    XRef, XRefId, XRefStatus,
};
use crate::render::{HighlightIndex, LineSpan, RenderOptions};
use crate::store::SessionFolder;
use crate::ui::UiState;

mod hints;

const FOCUS_COLOR: Color = Color::LightGreen;
const AGENT_FOCUS_COLOR: Color = Color::LightBlue;
const INSPECTOR_COLOR: Color = Color::DarkGray;
const FOOTER_LABEL_COLOR: Color = Color::Gray;
const FOOTER_KEY_COLOR: Color = Color::Cyan;
const FOOTER_BRAND_COLOR: Color = Color::White;
const FOOTER_BRAND: &str = "🅽 🅴 🆁 🅴 🅸 🅳 ";
const NODE_HINT_CHARS: &str = "ASDFJKLEWCMPGH";
const CENTER_BORDER_PADDING: i32 = 1;
const TUI_FLOWCHART_EXTRA_COL_GAP: usize = 2;

/// Runs the interactive terminal UI.
///
/// This is an app shell for T001; it renders a static diagram buffer and supports basic scrolling.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    run_with_session(demo_session())
}

pub fn run_with_session(session: crate::model::Session) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = TerminalSession::new()?;
    let mut app = App::new(session);

    while !app.should_quit {
        app.flush_pending_diagram_sync();
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_key(key);
                    if let Some(action) = app.take_external_action() {
                        let result =
                            terminal.run_external_action(|| app.execute_external_action(action));
                        if let Err(err) = result {
                            app.set_toast(format!("External action failed: {err}"));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

pub fn run_with_session_with_ui(
    session: crate::model::Session,
    agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_with_session_with_ui_state(session, agent_highlights, None, None)
}

pub fn run_with_session_with_ui_state(
    session: crate::model::Session,
    agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ui_state: Option<Arc<Mutex<UiState>>>,
    session_folder: Option<SessionFolder>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = TerminalSession::new()?;
    let mut app = App::new_with_ui(session, agent_highlights);
    app.ui_state = ui_state;
    app.session_folder = session_folder;
    app.publish_focus_to_ui_state();

    while !app.should_quit {
        app.sync_from_ui_state();
        app.flush_pending_diagram_sync();
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_key(key);
                    if let Some(action) = app.take_external_action() {
                        let result =
                            terminal.run_external_action(|| app.execute_external_action(action));
                        if let Err(err) = result {
                            app.set_toast(format!("External action failed: {err}"));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    let main_area = layout[0];
    let status_area = layout[1];

    let sidebar_panel_count = usize::from(app.objects_visible)
        + usize::from(app.xrefs_visible)
        + usize::from(app.inspector_visible);
    let compact_footer = footer_uses_compact_mode(main_area, sidebar_panel_count);
    let sidebar_panels_visible = sidebar_panel_count > 0;
    let (diagram_area, palette_area, sidebar_content_area) = if sidebar_panels_visible {
        let direction = if stack_main_panes_vertically(main_area, sidebar_panel_count) {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };
        let panes = Layout::default()
            .direction(direction)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(main_area);
        let diagram_area = panes[0];
        let sidebar_area = panes[1];

        let palette_height = if app.palette_visible { 2 } else { 0 };
        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(palette_height), Constraint::Min(0)])
            .split(sidebar_area);
        (diagram_area, Some(sidebar[0]), Some(sidebar[1]))
    } else {
        (main_area, None, None)
    };

    #[derive(Clone, Copy)]
    enum SidebarPanel {
        Objects,
        XRefs,
        Inspector,
    }
    let mut sidebar_panels = Vec::<SidebarPanel>::new();
    if app.objects_visible {
        sidebar_panels.push(SidebarPanel::Objects);
    }
    if app.xrefs_visible {
        sidebar_panels.push(SidebarPanel::XRefs);
    }
    if app.inspector_visible {
        sidebar_panels.push(SidebarPanel::Inspector);
    }

    let mut objects_area = None::<Rect>;
    let mut xrefs_area = None::<Rect>;
    let mut inspector_area = None::<Rect>;
    if !sidebar_panels.is_empty() {
        let Some(sidebar_content_area) = sidebar_content_area else {
            unreachable!("sidebar panels require a sidebar content area");
        };
        let constraints = match sidebar_panels.len() {
            1 => vec![Constraint::Min(0)],
            2 => vec![Constraint::Percentage(50), Constraint::Percentage(50)],
            _ => vec![
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(40),
            ],
        };
        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(sidebar_content_area);
        for (idx, panel) in sidebar_panels.into_iter().enumerate() {
            match panel {
                SidebarPanel::Objects => objects_area = Some(content[idx]),
                SidebarPanel::XRefs => xrefs_area = Some(content[idx]),
                SidebarPanel::Inspector => inspector_area = Some(content[idx]),
            }
        }
    }

    if app.palette_visible {
        if let Some(palette_area) = palette_area {
            let palette = Paragraph::new(Text::from(demo_palette_lines()));
            frame.render_widget(palette, palette_area);
        }
    }

    let active_diagram_id =
        app.active_diagram_id().map(ToString::to_string).unwrap_or_else(|| "—".to_owned());
    let diagram_ids = app.session.diagrams().keys().collect::<Vec<_>>();
    let diagram_total = diagram_ids.len();
    let diagram_index = app
        .active_diagram_id()
        .and_then(|active| diagram_ids.iter().position(|diagram_id| *diagram_id == active))
        .map(|idx| idx + 1);
    let diagram_title = diagram_view_title(
        &active_diagram_id,
        app.focus == Focus::Diagram,
        diagram_index,
        diagram_total,
    );
    let diagram_border_style =
        panel_border_style_for_focus(app.focus, Focus::Diagram, app.focus_owner);
    let viewport_width = diagram_area.width.saturating_sub(2) as usize;
    let viewport_height = diagram_area.height.saturating_sub(2) as usize;
    app.center_diagram_if_needed(viewport_width, viewport_height);
    let (scroll_x, scroll_y, left_pad, top_pad) = app.diagram_render_offsets();
    let mut diagram_text = app.diagram_text();
    if left_pad > 0 || top_pad > 0 {
        diagram_text = pad_text(diagram_text, left_pad, top_pad);
    }
    let diagram = Paragraph::new(diagram_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(diagram_title)
                .border_style(diagram_border_style),
        )
        .scroll((scroll_y, scroll_x));
    frame.render_widget(diagram, diagram_area);

    if let Some(objects_area) = objects_area {
        let objects_border_style =
            panel_border_style_for_focus(app.focus, Focus::Objects, app.focus_owner);
        let mut objects_suffix = None::<String>;
        if app.objects_selected_only {
            objects_suffix = Some("— selected only".to_owned());
        }
        let objects_title = view_title("Objects", '2', objects_suffix.as_deref());
        let marker_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
        let visible_objects = app.visible_object_indices();
        let selected_object_refs = app.session.selected_object_refs();
        let has_active_selection_in_objects = !selected_object_refs.is_empty();
        let objects_has_focus = app.focus == Focus::Objects;
        let cursor_visible_idx = app.objects_state.selected();
        let items = visible_objects
            .iter()
            .enumerate()
            .map(|(visible_idx, &idx)| {
                let obj = &app.objects[idx];
                let is_selected = selected_object_refs.contains(&obj.object_ref);
                let is_cursor = cursor_visible_idx == Some(visible_idx);
                let label_style = if has_active_selection_in_objects && !is_selected {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };

                let marker = if is_selected { "◼" } else { "◻" };
                let line = Line::from(vec![
                    Span::styled(marker, marker_style),
                    Span::raw(" "),
                    Span::styled(obj.label.clone(), label_style),
                ]);
                let mut item = ListItem::new(line);
                if let Some(bg) =
                    objects_item_bg(is_cursor, is_selected, objects_has_focus, app.focus_owner)
                {
                    item = item.style(Style::default().bg(bg));
                }
                item
            })
            .collect::<Vec<_>>();
        let objects_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(objects_title)
                    .border_style(objects_border_style),
            )
            .highlight_style(Style::default());
        frame.render_stateful_widget(objects_list, objects_area, &mut app.objects_state);
    }

    let selected_ref = app.selected_ref();
    if let Some(xrefs_area) = xrefs_area {
        let xrefs_border_style =
            panel_border_style_for_focus(app.focus, Focus::XRefs, app.focus_owner);
        let mut xrefs_title_suffix = Vec::new();
        if app.xrefs_dangling_only {
            xrefs_title_suffix.push("dangling only");
        }
        if app.xrefs_involving_only {
            xrefs_title_suffix.push("involving selection");
        }
        let xrefs_suffix = if xrefs_title_suffix.is_empty() {
            None
        } else {
            Some(format!("— {}", xrefs_title_suffix.join(", ")))
        };
        let xrefs_title = view_title("XRefs", '3', xrefs_suffix.as_deref());
        let visible_xrefs = app.visible_xref_indices();
        let xref_items = visible_xrefs
            .iter()
            .map(|&idx| {
                let xref = &app.xrefs[idx];
                let indirectly_selected = xref_involves_selected(selected_ref, &xref.xref);
                let style = xref_item_style(xref.xref.status(), indirectly_selected);
                let prefix = xref_direction_prefix(selected_ref, &xref.xref);
                ListItem::new(Line::from(vec![Span::raw(prefix), Span::raw(xref.label.clone())]))
                    .style(style)
            })
            .collect::<Vec<_>>();
        let xrefs_list = List::new(xref_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(xrefs_title)
                    .border_style(xrefs_border_style),
            )
            .highlight_style(xrefs_cursor_highlight_style(app.focus, app.focus_owner));
        frame.render_stateful_widget(xrefs_list, xrefs_area, &mut app.xrefs_state);
    }

    if let Some(inspector_area) = inspector_area {
        let (inspector_title, inspector_text) = match app.focus {
            Focus::XRefs => match app.selected_xref() {
                Some(selected) => {
                    let from_missing = !app.object_exists_in_session(selected.xref.from());
                    let to_missing = !app.object_exists_in_session(selected.xref.to());
                    (
                        view_title(
                            "Inspector",
                            '4',
                            Some(&format!(
                                "— XRef {} ({})",
                                selected.xref_id,
                                selected.xref.status()
                            )),
                        ),
                        format!(
                            "ID: {}\nKind: {}\nStatus: {}\nLabel: {}\nFrom: {}{}\nTo: {}{}",
                            selected.xref_id,
                            selected.xref.kind(),
                            selected.xref.status(),
                            selected.xref.label().unwrap_or("—"),
                            selected.xref.from(),
                            if from_missing { " (missing)" } else { "" },
                            selected.xref.to(),
                            if to_missing { " (missing)" } else { "" },
                        ),
                    )
                }
                None => (view_title("Inspector", '4', Some("— XRef")), "No selection".to_owned()),
            },
            _ => match app.selected_object() {
                Some(obj) => {
                    let category = obj.object_ref.category().segments().join("/");
                    (
                        view_title("Inspector", '4', Some(&format!("— {}", obj.object_ref))),
                        format!(
                            "Label: {}\nNote: {}\nRef: {}\nDiagram: {}\nCategory: {}\nObject: {}",
                            obj.label,
                            obj.note.as_deref().unwrap_or("—"),
                            obj.object_ref,
                            obj.object_ref.diagram_id(),
                            category,
                            obj.object_ref.object_id()
                        ),
                    )
                }
                None => (view_title("Inspector", '4', None), "No selection".to_owned()),
            },
        };
        let inspector = Paragraph::new(inspector_text)
            .style(Style::default().fg(INSPECTOR_COLOR))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(INSPECTOR_COLOR))
                    .title(inspector_title),
            );
        frame.render_widget(inspector, inspector_area);
    }

    let toast_snapshot = app.toast.as_ref().map(|toast| (toast.message.clone(), toast.expires_at));
    let toast_suffix = match toast_snapshot {
        Some((message, expires_at)) if expires_at > Instant::now() => format!(" | {message}"),
        Some(_) => {
            app.toast = None;
            String::new()
        }
        None => String::new(),
    };
    if app.search_mode != SearchMode::Inactive {
        let query = app.search_query.as_str();
        let status = Paragraph::new(search_footer_line(app, &toast_suffix));
        frame.render_widget(status, status_area);
        let brand = Paragraph::new(footer_brand_line()).alignment(Alignment::Right);
        frame.render_widget(brand, status_area);
        if app.search_mode == SearchMode::Editing {
            let cursor_x = status_area
                .x
                .saturating_add(1)
                .saturating_add(query.chars().count() as u16)
                .min(status_area.x.saturating_add(status_area.width.saturating_sub(1)));
            frame.set_cursor_position((cursor_x, status_area.y));
        }
        return;
    }

    let status = Paragraph::new(footer_help_line(app, &toast_suffix, compact_footer));
    frame.render_widget(status, status_area);
    let brand = Paragraph::new(footer_brand_line()).alignment(Alignment::Right);
    frame.render_widget(brand, status_area);

    if app.show_help {
        render_help(frame, app, main_area);
    }
}

// Extracted panel/header/footer/help rendering helpers.
include!("chrome.rs");

#[derive(Debug, Clone)]
struct SelectableObject {
    label: String,
    note: Option<String>,
    object_ref: ObjectRef,
}

#[derive(Debug, Clone)]
struct SelectableXRef {
    xref_id: XRefId,
    label: String,
    xref: XRef,
}

#[derive(Debug, Clone)]
struct Toast {
    message: String,
    expires_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchMode {
    Inactive,
    Editing,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchKind {
    Regular,
    Fuzzy,
}

#[derive(Debug, Clone)]
struct SearchCandidate {
    object_ref: ObjectRef,
    haystack: String,
}

#[derive(Debug, Clone)]
struct HintTarget {
    label: [char; 2],
    object_ref: ObjectRef,
    y: usize,
    inner_x0: usize,
    inner_x1: usize,
    fill_char: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HintKind {
    Jump,
    SelectChain,
}

#[derive(Debug, Clone, Default)]
enum HintMode {
    #[default]
    Inactive,
    AwaitingFirst {
        kind: HintKind,
        targets: Vec<HintTarget>,
    },
    AwaitingSecond {
        kind: HintKind,
        first: char,
        targets: Vec<HintTarget>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExternalAction {
    EditActiveDiagram,
}

#[derive(Debug, Clone)]
struct PendingDiagramSync {
    diagram_id: DiagramId,
    expected_disk_rev: u64,
}

struct App {
    session: Session,
    session_folder: Option<SessionFolder>,
    base_diagram: String,
    base_highlight_index: HighlightIndex,
    show_notes: bool,
    hint_mode: HintMode,
    hint_select_chain_prev: Option<ObjectRef>,
    pan_x: i32,
    pan_y: i32,
    center_diagram_on_next_draw: bool,
    focus: Focus,
    focus_owner: FocusOwner,
    ui_state: Option<Arc<Mutex<UiState>>>,
    ui_state_rev: u64,
    ui_state_session_rev: u64,
    agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    objects: Vec<SelectableObject>,
    objects_state: ListState,
    visible_object_indices: Vec<usize>,
    objects_visible: bool,
    objects_selected_only: bool,
    xrefs: Vec<SelectableXRef>,
    xrefs_state: ListState,
    visible_xref_indices: Vec<usize>,
    xrefs_visible: bool,
    xrefs_dangling_only: bool,
    xrefs_involving_only: bool,
    inspector_visible: bool,
    palette_visible: bool,
    follow_ai: bool,
    show_help: bool,
    help_scroll: u16,
    help_viewport_height: u16,
    toast: Option<Toast>,
    search_mode: SearchMode,
    search_kind: SearchKind,
    search_query: String,
    search_candidates: Vec<SearchCandidate>,
    search_results: Vec<ObjectRef>,
    search_result_index: usize,
    pending_external_action: Option<ExternalAction>,
    pending_diagram_sync: Option<PendingDiagramSync>,
    should_quit: bool,
}

impl App {
    fn new(session: Session) -> Self {
        Self::new_with_ui(session, Arc::new(Mutex::new(BTreeSet::new())))
    }

    fn new_with_ui(
        mut session: Session,
        agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ) -> Self {
        ensure_active_diagram_id(&mut session);

        let (base_diagram, base_highlight_index, objects) = match session
            .active_diagram_id()
            .and_then(|diagram_id| session.diagrams().get(diagram_id))
        {
            Some(diagram) => {
                let (text, highlight_index) =
                    render_diagram_annotated_for_tui(&session, diagram, true);
                (text, highlight_index, objects_from_diagram(diagram))
            }
            None => ("No diagrams in session".to_owned(), HighlightIndex::new(), Vec::new()),
        };

        let mut objects_state = ListState::default();
        if !objects.is_empty() {
            objects_state.select(Some(0));
        }
        let visible_object_indices: Vec<usize> = (0..objects.len()).collect();

        let xrefs = xrefs_from_session(&session);
        let mut xrefs_state = ListState::default();
        if !xrefs.is_empty() {
            xrefs_state.select(Some(0));
        }
        let visible_xref_indices: Vec<usize> = (0..xrefs.len()).collect();
        Self {
            session,
            session_folder: None,
            base_diagram,
            base_highlight_index,
            show_notes: true,
            hint_mode: HintMode::Inactive,
            hint_select_chain_prev: None,
            pan_x: 0,
            pan_y: 0,
            center_diagram_on_next_draw: true,
            focus: Focus::Diagram,
            focus_owner: FocusOwner::Human,
            ui_state: None,
            ui_state_rev: 0,
            ui_state_session_rev: 0,
            agent_highlights,
            objects,
            objects_state,
            visible_object_indices,
            objects_visible: false,
            objects_selected_only: false,
            xrefs,
            xrefs_state,
            visible_xref_indices,
            xrefs_visible: false,
            xrefs_dangling_only: false,
            xrefs_involving_only: false,
            inspector_visible: false,
            palette_visible: false,
            follow_ai: true,
            show_help: false,
            help_scroll: 0,
            help_viewport_height: 0,
            toast: None,
            search_mode: SearchMode::Inactive,
            search_kind: SearchKind::Regular,
            search_query: String::new(),
            search_candidates: Vec::new(),
            search_results: Vec::new(),
            search_result_index: 0,
            pending_external_action: None,
            pending_diagram_sync: None,
            should_quit: false,
        }
    }

    fn active_diagram_id(&self) -> Option<&DiagramId> {
        self.session.active_diagram_id()
    }

    fn publish_focus_to_ui_state(&mut self) {
        let Some(ui_state) = self.ui_state.as_ref() else {
            return;
        };

        let mut ui_state = ui_state.blocking_lock();
        ui_state.set_follow_ai(self.follow_ai);
        if self.focus_owner == FocusOwner::Human {
            let active_diagram_id = self.session.active_diagram_id().cloned();
            let active_object_ref = self.selected_ref().cloned();
            ui_state.set_human_selection(active_diagram_id, active_object_ref);
        }
    }

    fn sync_from_ui_state(&mut self) {
        if let Some(ui_state) = self.ui_state.as_ref() {
            let snapshot = ui_state.blocking_lock().clone();
            if snapshot.rev() != self.ui_state_rev {
                self.ui_state_rev = snapshot.rev();
                self.follow_ai = snapshot.follow_ai();
            }

            if snapshot.session_rev() != self.ui_state_session_rev
                && self.pending_diagram_sync.is_none()
            {
                match self.sync_session_from_disk() {
                    Ok(()) => {
                        self.ui_state_session_rev = snapshot.session_rev();
                    }
                    Err(_err) => {
                        // Keep the old session marker so the next tick retries reload.
                    }
                }
            }
        }

        if !self.follow_ai {
            return;
        }

        self.follow_agent_highlight();
    }

    fn sync_session_from_disk(&mut self) -> Result<(), String> {
        let Some(session_folder) = self.session_folder.as_ref() else {
            return Ok(());
        };

        let previous_selection = self.selected_ref().cloned();
        let mut disk_session = session_folder
            .load_session()
            .map_err(|err| format!("failed to reload session from disk: {err}"))?;
        ensure_active_diagram_id(&mut disk_session);

        if disk_session == self.session {
            return Ok(());
        }

        self.session = disk_session;
        self.retain_existing_selected_refs();
        self.refresh_xref_statuses();
        self.xrefs = xrefs_from_session(&self.session);
        self.apply_xref_filters();
        self.refresh_active_diagram_view();

        if let Some(object_ref) = previous_selection {
            self.select_object_ref(&object_ref);
        }

        Ok(())
    }

    fn follow_agent_highlight(&mut self) -> bool {
        let Some(object_ref) = self.agent_highlights.blocking_lock().iter().next().cloned() else {
            return false;
        };
        if !self.object_exists_in_session(&object_ref) {
            return false;
        }

        let already_selected = self.selected_ref().is_some_and(|selected| selected == &object_ref);
        let diagram_matches = self
            .active_diagram_id()
            .is_some_and(|diagram_id| diagram_id == object_ref.diagram_id());
        self.focus_owner = FocusOwner::Agent;
        if !already_selected || !diagram_matches {
            self.jump_to_object_ref(&object_ref);
        }
        true
    }

    fn refresh_active_diagram_view(&mut self) {
        self.cancel_hint_mode();
        let (base_diagram, base_highlight_index, objects) = match self
            .session
            .active_diagram_id()
            .and_then(|diagram_id| self.session.diagrams().get(diagram_id))
        {
            Some(diagram) => {
                let (text, highlight_index) =
                    render_diagram_annotated_for_tui(&self.session, diagram, self.show_notes);
                (text, highlight_index, objects_from_diagram(diagram))
            }
            None => ("No diagrams in session".to_owned(), HighlightIndex::new(), Vec::new()),
        };

        self.base_diagram = base_diagram;
        self.base_highlight_index = base_highlight_index;
        self.center_diagram_on_next_draw = true;
        self.pan_x = 0;
        self.pan_y = 0;

        self.objects = objects;
        self.recompute_visible_object_indices();
        let mut objects_state = ListState::default();
        if !self.visible_object_indices.is_empty() {
            objects_state.select(Some(0));
        }
        self.objects_state = objects_state;
        if self.xrefs_involving_only {
            self.apply_xref_filters();
        }
        self.publish_focus_to_ui_state();
    }

    fn rerender_active_diagram_buffer(&mut self) {
        self.cancel_hint_mode();
        let (base_diagram, base_highlight_index) = match self
            .session
            .active_diagram_id()
            .and_then(|diagram_id| self.session.diagrams().get(diagram_id))
        {
            Some(diagram) => {
                render_diagram_annotated_for_tui(&self.session, diagram, self.show_notes)
            }
            None => ("No diagrams in session".to_owned(), HighlightIndex::new()),
        };

        self.base_diagram = base_diagram;
        self.base_highlight_index = base_highlight_index;
    }

    fn center_diagram_if_needed(&mut self, viewport_width: usize, viewport_height: usize) {
        if !self.center_diagram_on_next_draw {
            return;
        }
        if viewport_width == 0 || viewport_height == 0 {
            return;
        }

        let diagram_width =
            self.base_diagram.split('\n').map(|line| line.chars().count()).max().unwrap_or(0)
                as i32;
        let raw_line_count = self.base_diagram.split('\n').count() as i32;
        let diagram_height = raw_line_count.max(0);
        let viewport_width = viewport_width as i32;
        let viewport_height = viewport_height as i32;

        let centered_pan_x = (diagram_width - viewport_width) / 2;
        let centered_pan_y = (diagram_height - viewport_height) / 2;
        let max_pan = -CENTER_BORDER_PADDING;
        // Never start clipped on the left/top; when full centering would do that, align with a
        // one-cell margin to the diagram border.
        self.pan_x = centered_pan_x.min(max_pan);
        self.pan_y = centered_pan_y.min(max_pan);
        self.center_diagram_on_next_draw = false;
    }

    fn diagram_render_offsets(&self) -> (u16, u16, usize, usize) {
        let scroll_x = clamp_positive_i32_to_u16(self.pan_x);
        let scroll_y = clamp_positive_i32_to_u16(self.pan_y);
        let left_pad = self.pan_x.saturating_neg().max(0) as usize;
        let top_pad = self.pan_y.saturating_neg().max(0) as usize;
        (scroll_x, scroll_y, left_pad, top_pad)
    }

    fn toggle_show_notes(&mut self) {
        self.show_notes = !self.show_notes;
        self.rerender_active_diagram_buffer();
    }

    fn set_active_diagram_id(&mut self, diagram_id: DiagramId) {
        self.cancel_hint_mode();
        self.session.set_active_diagram_id(Some(diagram_id));
        if let Some(session_folder) = self.session_folder.as_ref() {
            if let Err(err) = session_folder.save_active_diagram_id(&self.session) {
                self.set_toast(format!("Active diagram persist failed: {err}"));
            }
        }
        self.refresh_active_diagram_view();
    }

    fn switch_diagram_prev(&mut self) {
        self.cancel_hint_mode();
        let diagram_ids: Vec<DiagramId> = self.session.diagrams().keys().cloned().collect();
        if diagram_ids.is_empty() {
            return;
        }

        let current_idx = self
            .session
            .active_diagram_id()
            .and_then(|active| diagram_ids.iter().position(|id| id == active))
            .unwrap_or(0);
        let prev_idx = match current_idx {
            0 => diagram_ids.len().saturating_sub(1),
            n => n - 1,
        };

        self.set_active_diagram_id(diagram_ids[prev_idx].clone());
    }

    fn switch_diagram_next(&mut self) {
        self.cancel_hint_mode();
        let diagram_ids: Vec<DiagramId> = self.session.diagrams().keys().cloned().collect();
        if diagram_ids.is_empty() {
            return;
        }

        let current_idx = self
            .session
            .active_diagram_id()
            .and_then(|active| diagram_ids.iter().position(|id| id == active))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % diagram_ids.len();

        self.set_active_diagram_id(diagram_ids[next_idx].clone());
    }

    fn visible_object_indices(&self) -> &[usize] {
        &self.visible_object_indices
    }

    fn selected_object_index(&self) -> Option<usize> {
        let visible_idx = self.objects_state.selected()?;
        self.visible_object_indices.get(visible_idx).copied()
    }

    fn selected_object(&self) -> Option<&SelectableObject> {
        let idx = self.selected_object_index()?;
        self.objects.get(idx)
    }

    fn selected_ref(&self) -> Option<&ObjectRef> {
        self.selected_object().map(|obj| &obj.object_ref)
    }

    fn diagram_text(&self) -> Text<'static> {
        let selected_ref = self.selected_ref().cloned();
        let agent_highlight = self.agent_highlights.blocking_lock().iter().next().cloned();
        let (hint_first_typed, hint_targets) = match &self.hint_mode {
            HintMode::Inactive => (None, &[][..]),
            HintMode::AwaitingFirst { targets, .. } => (None, targets.as_slice()),
            HintMode::AwaitingSecond { first, targets, .. } => (Some(*first), targets.as_slice()),
        };

        let mut flags_by_line = self
            .base_diagram
            .split('\n')
            .map(|line| vec![0u8; line.chars().count()])
            .collect::<Vec<_>>();
        let mut sequence_block_cells_by_line = self
            .base_diagram
            .split('\n')
            .map(|line| vec![false; line.chars().count()])
            .collect::<Vec<_>>();
        let mut sequence_area_cells_by_line = self
            .base_diagram
            .split('\n')
            .map(|line| vec![false; line.chars().count()])
            .collect::<Vec<_>>();
        let mut note_cells_by_line = self
            .base_diagram
            .split('\n')
            .map(|line| vec![false; line.chars().count()])
            .collect::<Vec<_>>();
        for (object_ref, spans) in &self.base_highlight_index {
            if is_sequence_block_or_section_ref(object_ref) {
                apply_presence_flags(&mut sequence_block_cells_by_line, spans);
            }
            if is_sequence_section_ref(object_ref) {
                apply_bounding_area_flags(&mut sequence_area_cells_by_line, spans);
            }
            if is_note_ref(object_ref) {
                apply_presence_flags(&mut note_cells_by_line, spans);
            }
        }
        let sequence_block_color = Color::LightYellow;
        let sequence_area_bg = Color::Yellow;

        if let Some(selected_ref) = selected_ref.as_ref() {
            if let Some(spans) = self.base_highlight_index.get(selected_ref) {
                apply_highlight_flags(&mut flags_by_line, spans, 0b01);
                fill_highlight_bridge_gaps(&mut flags_by_line, &self.base_diagram, 0b01);
                fill_highlight_text_space_gaps(&mut flags_by_line, &self.base_diagram, 0b01);
                if is_flow_edge_ref(selected_ref) {
                    fill_highlight_bridge_gaps_unbounded(
                        &mut flags_by_line,
                        &self.base_diagram,
                        0b01,
                    );
                }
                // Keep cursor highlight tight around the focused object; corner extension is
                // reserved for selected-set rendering to avoid node connection overdraw.
            }
        }

        let has_selected_flow_edge =
            self.session.selected_object_refs().iter().any(is_flow_edge_ref);
        let mut has_selected_objects_in_diagram = false;
        for object_ref in self.session.selected_object_refs() {
            if let Some(spans) = self.base_highlight_index.get(object_ref) {
                has_selected_objects_in_diagram = true;
                apply_highlight_flags(&mut flags_by_line, spans, 0b100);
            }
        }
        if has_selected_objects_in_diagram {
            fill_highlight_bridge_gaps(&mut flags_by_line, &self.base_diagram, 0b100);
            fill_highlight_text_space_gaps(&mut flags_by_line, &self.base_diagram, 0b100);
            if has_selected_flow_edge {
                fill_highlight_bridge_gaps_unbounded(&mut flags_by_line, &self.base_diagram, 0b100);
            }
            if !has_selected_flow_edge {
                fill_highlight_corner_branch_extensions(
                    &mut flags_by_line,
                    &self.base_diagram,
                    0b100,
                );
            }
        }

        if let Some(object_ref) = agent_highlight {
            if let Some(spans) = self.base_highlight_index.get(&object_ref) {
                apply_highlight_flags(&mut flags_by_line, spans, 0b10);
                fill_highlight_text_space_gaps(&mut flags_by_line, &self.base_diagram, 0b10);
            }
        }
        let has_active_selection_in_diagram = has_selected_objects_in_diagram;

        let mut out = Text::default();
        for (y, line) in self.base_diagram.split('\n').enumerate() {
            let mut chars = line.chars().collect::<Vec<_>>();
            let mut flags = flags_by_line.get(y).cloned().unwrap_or_default();
            let mut style_overrides = vec![None::<Style>; chars.len()];

            if !hint_targets.is_empty() {
                for target in hint_targets.iter().filter(|target| target.y == y) {
                    apply_hint_target_to_line(
                        &mut chars,
                        &mut flags,
                        &mut style_overrides,
                        target,
                        hint_first_typed,
                    );
                }
            }

            let mut line_spans = Vec::<Span<'static>>::new();
            if chars.is_empty() {
                line_spans.push(Span::raw(String::new()));
            } else {
                let mut current_style =
                    style_overrides.first().and_then(|style| *style).unwrap_or_else(|| {
                        style_for_diagram_cell(
                            flags.first().copied().unwrap_or(0),
                            has_active_selection_in_diagram,
                            self.focus_owner,
                            note_cells_by_line
                                .get(y)
                                .and_then(|line| line.first())
                                .copied()
                                .unwrap_or(false),
                            sequence_block_cells_by_line
                                .get(y)
                                .and_then(|line| line.first())
                                .copied()
                                .unwrap_or(false),
                            sequence_block_color,
                            sequence_area_cells_by_line
                                .get(y)
                                .and_then(|line| line.first())
                                .copied()
                                .unwrap_or(false),
                            sequence_area_bg,
                        )
                    });
                current_style = style_for_diagram_char(current_style, chars[0]);
                let mut buf = String::new();

                for (idx, ch) in chars.iter().enumerate() {
                    let base_style =
                        style_overrides.get(idx).and_then(|style| *style).unwrap_or_else(|| {
                            style_for_diagram_cell(
                                flags.get(idx).copied().unwrap_or(0),
                                has_active_selection_in_diagram,
                                self.focus_owner,
                                note_cells_by_line
                                    .get(y)
                                    .and_then(|line| line.get(idx))
                                    .copied()
                                    .unwrap_or(false),
                                sequence_block_cells_by_line
                                    .get(y)
                                    .and_then(|line| line.get(idx))
                                    .copied()
                                    .unwrap_or(false),
                                sequence_block_color,
                                sequence_area_cells_by_line
                                    .get(y)
                                    .and_then(|line| line.get(idx))
                                    .copied()
                                    .unwrap_or(false),
                                sequence_area_bg,
                            )
                        });
                    let style = style_for_diagram_char(base_style, *ch);
                    if style != current_style {
                        if !buf.is_empty() {
                            line_spans.push(Span::styled(buf, current_style));
                            buf = String::new();
                        }
                        current_style = style;
                    }
                    buf.push(*ch);
                }

                if !buf.is_empty() {
                    line_spans.push(Span::styled(buf, current_style));
                } else {
                    line_spans.push(Span::raw(String::new()));
                }
            }

            out.lines.push(Line::from(line_spans));
        }

        out
    }

    fn recompute_visible_object_indices(&mut self) {
        self.visible_object_indices.clear();
        for (idx, obj) in self.objects.iter().enumerate() {
            if self.objects_selected_only
                && !self.session.selected_object_refs().contains(&obj.object_ref)
            {
                continue;
            }
            self.visible_object_indices.push(idx);
        }
    }

    fn apply_object_filters(&mut self) {
        let prev_selected_visible = self.objects_state.selected();
        let prev_selected_ref = self.selected_ref().cloned();

        self.recompute_visible_object_indices();

        let visible = self.visible_object_indices();
        if visible.is_empty() {
            self.objects_state.select(None);
            if self.xrefs_involving_only {
                self.apply_xref_filters();
            }
            self.publish_focus_to_ui_state();
            return;
        }

        let next_selected = prev_selected_ref
            .and_then(|prev| self.object_index_for_ref(&prev))
            .and_then(|prev| visible.iter().position(|&idx| idx == prev))
            .or_else(|| prev_selected_visible.map(|idx| idx.min(visible.len().saturating_sub(1))))
            .unwrap_or(0);

        self.objects_state.select(Some(next_selected));
        if self.xrefs_involving_only {
            self.apply_xref_filters();
        }
        self.publish_focus_to_ui_state();
    }

    fn recompute_visible_xref_indices(&mut self) {
        let involving_ref = self.selected_ref().cloned();
        self.visible_xref_indices.clear();
        for (idx, xref) in self.xrefs.iter().enumerate() {
            if self.xrefs_dangling_only && !xref.xref.status().is_dangling() {
                continue;
            }

            if self.xrefs_involving_only {
                let Some(ref involving_ref) = involving_ref else {
                    continue;
                };
                if xref.xref.from() != involving_ref && xref.xref.to() != involving_ref {
                    continue;
                }
            }

            self.visible_xref_indices.push(idx);
        }
    }

    fn apply_xref_filters(&mut self) {
        let prev_selected = self.selected_xref_index();
        self.recompute_visible_xref_indices();

        let visible = self.visible_xref_indices();
        if visible.is_empty() {
            self.xrefs_state.select(None);
            return;
        }

        let next_selected =
            prev_selected.and_then(|prev| visible.iter().position(|&idx| idx == prev)).unwrap_or(0);
        self.xrefs_state.select(Some(next_selected));
    }

    fn visible_xref_indices(&self) -> &[usize] {
        &self.visible_xref_indices
    }

    fn selected_xref_index(&self) -> Option<usize> {
        let visible_idx = self.xrefs_state.selected()?;
        self.visible_xref_indices.get(visible_idx).copied()
    }

    fn selected_xref(&self) -> Option<&SelectableXRef> {
        let idx = self.selected_xref_index()?;
        self.xrefs.get(idx)
    }

    fn toggle_xrefs_dangling_only(&mut self) {
        self.xrefs_dangling_only = !self.xrefs_dangling_only;
        self.apply_xref_filters();
    }

    fn toggle_xrefs_involving_only(&mut self) {
        self.xrefs_involving_only = !self.xrefs_involving_only;
        self.apply_xref_filters();
    }

    fn toggle_inspector_visible(&mut self) {
        self.inspector_visible = !self.inspector_visible;
        self.set_toast(if self.inspector_visible { "Inspector shown" } else { "Inspector hidden" });
    }

    fn toggle_palette_visible(&mut self) {
        self.palette_visible = !self.palette_visible;
        self.set_toast(if self.palette_visible { "Palette shown" } else { "Palette hidden" });
    }

    fn panel_is_visible(&self, focus: Focus) -> bool {
        match focus {
            Focus::Diagram => true,
            Focus::Objects => self.objects_visible,
            Focus::XRefs => self.xrefs_visible,
        }
    }

    fn ensure_focus_visible(&mut self) {
        if self.panel_is_visible(self.focus) {
            return;
        }
        self.focus = if self.objects_visible {
            Focus::Objects
        } else if self.xrefs_visible {
            Focus::XRefs
        } else {
            Focus::Diagram
        };
    }

    fn cycle_focus_visible(&mut self) {
        let mut next = self.focus;
        for _ in 0..3 {
            next = next.cycle();
            if self.panel_is_visible(next) {
                self.focus = next;
                return;
            }
        }
        self.focus = Focus::Diagram;
    }

    fn cycle_focus_visible_back(&mut self) {
        let mut next = self.focus;
        for _ in 0..3 {
            next = next.cycle_back();
            if self.panel_is_visible(next) {
                self.focus = next;
                return;
            }
        }
        self.focus = Focus::Diagram;
    }

    fn toggle_objects_visible_and_focus(&mut self) {
        self.objects_visible = !self.objects_visible;
        if self.objects_visible {
            self.focus = Focus::Objects;
            self.set_toast("Objects shown");
        } else {
            self.ensure_focus_visible();
            self.set_toast("Objects hidden");
        }
    }

    fn toggle_xrefs_visible_and_focus(&mut self) {
        self.xrefs_visible = !self.xrefs_visible;
        if self.xrefs_visible {
            self.focus = Focus::XRefs;
            self.set_toast("XRefs shown");
        } else {
            self.ensure_focus_visible();
            self.set_toast("XRefs hidden");
        }
    }

    fn object_index_for_ref(&self, object_ref: &ObjectRef) -> Option<usize> {
        self.objects.iter().position(|obj| &obj.object_ref == object_ref)
    }

    fn object_exists_in_session(&self, object_ref: &ObjectRef) -> bool {
        self.session.object_ref_exists(object_ref)
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.handle_key_code(key.code) {
            self.should_quit = true;
        }
    }

    fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            if self.search_mode != SearchMode::Inactive {
                self.clear_search();
            }
            self.cancel_hint_mode();
            self.help_scroll = 0;
        }
    }

    fn toggle_follow_ai(&mut self) {
        self.follow_ai = !self.follow_ai;
        self.publish_focus_to_ui_state();
        if self.follow_ai {
            self.follow_agent_highlight();
        }
        self.set_toast(if self.follow_ai { "Follow AI enabled" } else { "Follow AI disabled" });
    }

    fn take_external_action(&mut self) -> Option<ExternalAction> {
        self.pending_external_action.take()
    }

    fn queue_edit_active_diagram(&mut self) {
        self.pending_external_action = Some(ExternalAction::EditActiveDiagram);
    }

    fn execute_external_action(&mut self, action: ExternalAction) -> Result<(), String> {
        match action {
            ExternalAction::EditActiveDiagram => self.edit_active_diagram_in_editor(),
        }
    }

    fn edit_active_diagram_in_editor(&mut self) -> Result<(), String> {
        let Some(diagram_id) = self.active_diagram_id().cloned() else {
            return Err("no active diagram".to_owned());
        };
        let Some(diagram) = self.session.diagrams().get(&diagram_id).cloned() else {
            return Err(format!("diagram not found: {diagram_id}"));
        };

        let original_mermaid = export_diagram_mermaid(&diagram)?;
        let temp_path = write_temp_mermaid_file(&diagram_id, &original_mermaid)?;
        let editor_command = resolve_editor_command();

        let launch_result = launch_editor_command(&editor_command, &temp_path);
        let edited_mermaid = fs::read_to_string(&temp_path).map_err(|err| {
            format!("failed reading edited Mermaid from {}: {err}", temp_path.display())
        });
        let _ = fs::remove_file(&temp_path);

        launch_result?;
        let edited_mermaid = edited_mermaid?;

        if edited_mermaid == original_mermaid {
            self.set_toast(format!("Edit cancelled (no changes): {diagram_id}"));
            return Ok(());
        }

        self.apply_edited_mermaid_to_diagram(
            &diagram_id,
            diagram.kind(),
            diagram.rev(),
            &edited_mermaid,
        )
    }

    fn apply_edited_mermaid_to_diagram(
        &mut self,
        diagram_id: &DiagramId,
        diagram_kind: DiagramKind,
        baseline_rev: u64,
        mermaid: &str,
    ) -> Result<(), String> {
        let parsed_ast = parse_mermaid_for_kind(diagram_kind, mermaid)?;
        let Some(current_diagram) = self.session.diagrams().get(diagram_id) else {
            return Err(format!("diagram not found: {diagram_id}"));
        };
        if current_diagram.ast() == &parsed_ast {
            self.set_toast(format!("No structural changes: {diagram_id}"));
            return Ok(());
        }

        {
            let Some(diagram) = self.session.diagrams_mut().get_mut(diagram_id) else {
                return Err(format!("diagram not found: {diagram_id}"));
            };
            diagram
                .set_ast(parsed_ast)
                .map_err(|err| format!("failed applying edited Mermaid: {err}"))?;
            diagram.bump_rev();
        }

        self.retain_existing_selected_refs();
        self.refresh_xref_statuses();
        self.xrefs = xrefs_from_session(&self.session);
        self.apply_xref_filters();
        self.refresh_active_diagram_view();

        let new_rev = self
            .session
            .diagrams()
            .get(diagram_id)
            .map(|diagram| diagram.rev())
            .unwrap_or(baseline_rev);

        if self.session_folder.is_some() {
            self.pending_diagram_sync = Some(PendingDiagramSync {
                diagram_id: diagram_id.clone(),
                expected_disk_rev: baseline_rev,
            });
            self.set_toast(format!(
                "Edited {diagram_id} (rev {baseline_rev}->{new_rev}); sync pending"
            ));
        } else {
            self.set_toast(format!("Edited {diagram_id} (rev {baseline_rev}->{new_rev})"));
        }

        Ok(())
    }

    fn flush_pending_diagram_sync(&mut self) {
        let Some(pending) = self.pending_diagram_sync.take() else {
            return;
        };
        let Some(session_folder) = self.session_folder.clone() else {
            return;
        };

        match self.persist_pending_diagram_sync(&session_folder, &pending) {
            Ok(()) => {
                self.set_toast(format!("Synced edited diagram: {}", pending.diagram_id));
            }
            Err(err) => {
                self.set_toast(err);
            }
        }
    }

    fn persist_pending_diagram_sync(
        &self,
        session_folder: &SessionFolder,
        pending: &PendingDiagramSync,
    ) -> Result<(), String> {
        let Some(local_diagram) = self.session.diagrams().get(&pending.diagram_id).cloned() else {
            return Err(format!(
                "sync skipped: edited diagram no longer exists: {}",
                pending.diagram_id
            ));
        };

        let mut disk_session =
            session_folder.load_session().map_err(|err| format!("sync failed (load): {err}"))?;
        let Some(disk_diagram) = disk_session.diagrams().get(&pending.diagram_id) else {
            return Err(format!("sync conflict: diagram removed on disk: {}", pending.diagram_id));
        };

        if disk_diagram.rev() != pending.expected_disk_rev {
            return Err(format!(
                "sync conflict for {}: disk rev {} != expected {}",
                pending.diagram_id,
                disk_diagram.rev(),
                pending.expected_disk_rev
            ));
        }

        disk_session.diagrams_mut().insert(pending.diagram_id.clone(), local_diagram);
        session_folder
            .save_session(&disk_session)
            .map_err(|err| format!("sync failed (save): {err}"))
    }

    fn retain_existing_selected_refs(&mut self) {
        let retained = self
            .session
            .selected_object_refs()
            .iter()
            .filter(|object_ref| self.session.object_ref_exists(object_ref))
            .cloned()
            .collect::<BTreeSet<_>>();
        self.session.set_selected_object_refs(retained);
    }

    fn refresh_xref_statuses(&mut self) {
        let next_statuses = self
            .session
            .xrefs()
            .iter()
            .map(|(xref_id, xref)| {
                let from_dangling = self.session.object_ref_is_missing(xref.from());
                let to_dangling = self.session.object_ref_is_missing(xref.to());
                (xref_id.clone(), XRefStatus::from_flags(from_dangling, to_dangling))
            })
            .collect::<Vec<_>>();

        for (xref_id, status) in next_statuses {
            if let Some(xref) = self.session.xrefs_mut().get_mut(&xref_id) {
                xref.set_status(status);
            }
        }
    }

    fn help_scroll_by(&mut self, delta: i32) {
        if delta < 0 {
            self.help_scroll = self.help_scroll.saturating_sub((-delta) as u16);
        } else {
            self.help_scroll = self.help_scroll.saturating_add(delta as u16);
        }
    }

    fn help_scroll_page(&mut self, direction: i32) {
        let page = self.help_viewport_height.max(1).saturating_sub(1) as i32;
        let step = page.max(1);
        self.help_scroll_by(direction.signum() * step);
    }

    fn handle_key_code(&mut self, code: KeyCode) -> bool {
        self.focus_owner = FocusOwner::Human;

        if self.show_help {
            match code {
                KeyCode::Esc | KeyCode::Char('?') => {
                    self.show_help = false;
                }
                KeyCode::Char('q') => return true,
                KeyCode::Down | KeyCode::Char('j') => self.help_scroll_by(1),
                KeyCode::Up | KeyCode::Char('k') => self.help_scroll_by(-1),
                KeyCode::PageDown => self.help_scroll_page(1),
                KeyCode::PageUp => self.help_scroll_page(-1),
                KeyCode::Home => self.help_scroll = 0,
                KeyCode::End => self.help_scroll = u16::MAX,
                _ => {}
            }
            return false;
        }

        match self.search_mode {
            SearchMode::Editing => {
                self.handle_search_edit_key(code);
                return false;
            }
            SearchMode::Results => {
                if matches!(code, KeyCode::Esc) {
                    self.clear_search();
                    return false;
                }
            }
            SearchMode::Inactive => {}
        }

        if !matches!(code, KeyCode::Char('?') | KeyCode::Char('/') | KeyCode::Char('\\'))
            && matches!(self.focus, Focus::Diagram | Focus::Objects)
            && self.handle_diagram_hint_key(code)
        {
            return false;
        }

        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('1') => self.focus = Focus::Diagram,
            KeyCode::Char('2') => self.toggle_objects_visible_and_focus(),
            KeyCode::Char('3') => self.toggle_xrefs_visible_and_focus(),
            KeyCode::Char('4') => self.toggle_inspector_visible(),
            KeyCode::Char('|') => self.toggle_palette_visible(),
            KeyCode::Char('a') => self.toggle_follow_ai(),
            KeyCode::Char('d') => self.deselect_current_diagram_objects(),
            KeyCode::Char('/') => self.enter_search_mode(SearchKind::Regular),
            KeyCode::Char('\\') => self.enter_search_mode(SearchKind::Fuzzy),
            KeyCode::Char('?') => self.toggle_help(),
            KeyCode::Char('n') => {
                if self.search_mode == SearchMode::Inactive && self.focus == Focus::Diagram {
                    self.toggle_show_notes();
                } else {
                    self.search_next();
                }
            }
            KeyCode::Char('N') => self.search_prev(),
            KeyCode::Tab => {
                self.cycle_focus_visible();
            }
            KeyCode::BackTab => {
                self.cycle_focus_visible_back();
            }
            KeyCode::Char('[') => self.switch_diagram_prev(),
            KeyCode::Char(']') => self.switch_diagram_next(),

            _ => match self.focus {
                Focus::Diagram => self.handle_diagram_key(code),
                Focus::Objects => self.handle_objects_key(code),
                Focus::XRefs => self.handle_xrefs_key(code),
            },
        }

        false
    }

    fn enter_search_mode(&mut self, kind: SearchKind) {
        self.search_mode = SearchMode::Editing;
        self.search_kind = kind;
        self.search_query.clear();
        self.search_result_index = 0;
        self.search_results.clear();
        self.search_candidates = search_candidates_from_session(&self.session);
    }

    fn handle_search_edit_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.clear_search(),
            KeyCode::Enter => self.commit_search(),
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search_results();
            }
            KeyCode::Char(ch) => {
                self.search_query.push(ch);
                self.update_search_results();
            }
            _ => {}
        }
    }

    fn commit_search(&mut self) {
        if self.search_results.len() > 1 {
            self.search_mode = SearchMode::Results;
        } else {
            self.search_mode = SearchMode::Inactive;
        }
    }

    fn clear_search(&mut self) {
        self.search_mode = SearchMode::Inactive;
        self.search_query.clear();
        self.search_candidates.clear();
        self.search_results.clear();
        self.search_result_index = 0;
    }

    fn update_search_results(&mut self) {
        self.search_results = ranked_search_results(
            &self.search_candidates,
            &self.search_query,
            self.search_kind,
            self.active_diagram_id(),
        );
        self.search_result_index = 0;
        self.jump_to_current_search_result();
    }

    fn search_prefix(&self) -> char {
        match self.search_kind {
            SearchKind::Regular => '/',
            SearchKind::Fuzzy => '\\',
        }
    }

    fn jump_to_current_search_result(&mut self) {
        let Some(object_ref) = self.search_results.get(self.search_result_index).cloned() else {
            return;
        };
        self.select_object_ref(&object_ref);
    }

    fn search_next(&mut self) {
        let len = self.search_results.len();
        if len <= 1 {
            return;
        }

        self.search_result_index = (self.search_result_index + 1) % len;
        self.jump_to_current_search_result();
    }

    fn search_prev(&mut self) {
        let len = self.search_results.len();
        if len <= 1 {
            return;
        }

        self.search_result_index = match self.search_result_index {
            0 => len - 1,
            n => n - 1,
        };
        self.jump_to_current_search_result();
    }

    fn handle_diagram_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('f') => self.enter_diagram_hint_mode(),
            KeyCode::Char('c') => self.enter_diagram_select_hint_mode(),
            KeyCode::Char('e') => self.queue_edit_active_diagram(),
            KeyCode::Char(' ') => self.toggle_selected_object(),
            KeyCode::Char('y') => self.yank_selected_object_ref(),
            KeyCode::Char('g') => self.jump_to_selected_incoming_xref(),
            KeyCode::Char('t') => self.jump_to_selected_outgoing_xref(),
            KeyCode::Up | KeyCode::Char('k') => self.pan_y = self.pan_y.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.pan_y = self.pan_y.saturating_add(1),
            KeyCode::Left | KeyCode::Char('h') => self.pan_x = self.pan_x.saturating_sub(1),
            KeyCode::Right | KeyCode::Char('l') => self.pan_x = self.pan_x.saturating_add(1),
            KeyCode::Char('K') => self.pan_y = self.pan_y.saturating_sub(10),
            KeyCode::Char('J') => self.pan_y = self.pan_y.saturating_add(10),
            KeyCode::Char('H') => self.pan_x = self.pan_x.saturating_sub(10),
            KeyCode::Char('L') => self.pan_x = self.pan_x.saturating_add(10),

            KeyCode::PageUp => self.pan_y = self.pan_y.saturating_sub(10),
            KeyCode::PageDown => self.pan_y = self.pan_y.saturating_add(10),
            KeyCode::Home => {
                self.pan_x = 0;
                self.pan_y = 0;
            }
            KeyCode::End => {
                self.pan_x = i32::MAX;
                self.pan_y = i32::MAX;
            }

            _ => {}
        }
    }

    fn cancel_hint_mode(&mut self) {
        self.hint_mode = HintMode::Inactive;
        self.hint_select_chain_prev = None;
    }

    fn handle_diagram_hint_key(&mut self, code: KeyCode) -> bool {
        let mode = std::mem::replace(&mut self.hint_mode, HintMode::Inactive);
        match mode {
            HintMode::Inactive => {
                self.hint_mode = HintMode::Inactive;
                false
            }
            HintMode::AwaitingFirst { kind, targets } => match code {
                KeyCode::Esc => {
                    self.cancel_hint_mode();
                    true
                }
                KeyCode::Char(ch) => {
                    let first = ch.to_ascii_uppercase();
                    let filtered = targets
                        .iter()
                        .filter(|target| target.label[0] == first)
                        .cloned()
                        .collect::<Vec<_>>();

                    if filtered.is_empty() {
                        match kind {
                            HintKind::Jump => {}
                            HintKind::SelectChain => {
                                self.hint_mode = HintMode::AwaitingFirst { kind, targets };
                                self.set_toast(format!("No hint targets for '{first}'"));
                            }
                        }
                    } else {
                        self.hint_mode =
                            HintMode::AwaitingSecond { kind, first, targets: filtered };
                    }
                    true
                }
                _ => {
                    self.hint_mode = HintMode::AwaitingFirst { kind, targets };
                    true
                }
            },
            HintMode::AwaitingSecond { kind, first, targets } => match code {
                KeyCode::Esc => {
                    self.cancel_hint_mode();
                    true
                }
                KeyCode::Char(ch) => {
                    let second = ch.to_ascii_uppercase();
                    let selected = targets
                        .iter()
                        .find(|target| target.label[0] == first && target.label[1] == second)
                        .map(|target| target.object_ref.clone());

                    match (kind, selected) {
                        (HintKind::Jump, Some(object_ref)) => {
                            self.select_object_ref(&object_ref);
                        }
                        (HintKind::Jump, None) => {}
                        (HintKind::SelectChain, Some(object_ref)) => {
                            self.select_chain_object_ref(&object_ref);
                            self.reset_diagram_hint_mode(HintKind::SelectChain);
                        }
                        (HintKind::SelectChain, None) => {
                            self.hint_mode = HintMode::AwaitingSecond { kind, first, targets };
                            self.set_toast("No matching hint".to_owned());
                        }
                    }

                    true
                }
                _ => {
                    self.hint_mode = HintMode::AwaitingSecond { kind, first, targets };
                    true
                }
            },
        }
    }

    fn reset_diagram_hint_mode(&mut self, kind: HintKind) {
        let Some(targets) = self.compute_diagram_hint_targets(kind) else {
            self.cancel_hint_mode();
            return;
        };
        self.hint_mode = HintMode::AwaitingFirst { kind, targets };
    }

    fn enter_diagram_hint_mode(&mut self) {
        let Some(diagram_id) = self.active_diagram_id().cloned() else {
            return;
        };
        if self.session.diagrams().get(&diagram_id).is_none() {
            return;
        };

        self.hint_select_chain_prev = None;
        self.reset_diagram_hint_mode(HintKind::Jump);
    }

    fn enter_diagram_select_hint_mode(&mut self) {
        let Some(diagram_id) = self.active_diagram_id().cloned() else {
            return;
        };
        if self.session.diagrams().get(&diagram_id).is_none() {
            return;
        }

        self.hint_select_chain_prev = None;
        self.reset_diagram_hint_mode(HintKind::SelectChain);
    }

    fn compute_diagram_hint_targets(&mut self, kind: HintKind) -> Option<Vec<HintTarget>> {
        let diagram_id = self.active_diagram_id().cloned()?;
        let diagram = self.session.diagrams().get(&diagram_id)?;

        let hintable_refs = match diagram.ast() {
            DiagramAst::Flowchart(ast) => {
                let mut refs = Vec::new();
                let node_category = category_path(&["flow", "node"]);
                let edge_category = category_path(&["flow", "edge"]);

                for node_id in ast.nodes().keys() {
                    refs.push(ObjectRef::new(
                        diagram_id.clone(),
                        node_category.clone(),
                        node_id.clone(),
                    ));
                }
                for edge_id in ast.edges().keys() {
                    refs.push(ObjectRef::new(
                        diagram_id.clone(),
                        edge_category.clone(),
                        edge_id.clone(),
                    ));
                }

                refs
            }
            DiagramAst::Sequence(ast) => {
                let mut refs = Vec::new();
                let participant_category = category_path(&["seq", "participant"]);

                for participant_id in ast.participants().keys() {
                    refs.push(ObjectRef::new(
                        diagram_id.clone(),
                        participant_category.clone(),
                        participant_id.clone(),
                    ));
                }

                if kind == HintKind::Jump {
                    let message_category = category_path(&["seq", "message"]);
                    for msg in ast.messages() {
                        refs.push(ObjectRef::new(
                            diagram_id.clone(),
                            message_category.clone(),
                            msg.message_id().clone(),
                        ));
                    }
                }

                refs
            }
        };

        let mut placements = Vec::<(ObjectRef, (usize, usize, usize, char))>::new();
        let lines: Vec<&str> = self.base_diagram.split('\n').collect();

        for object_ref in hintable_refs {
            let Some(spans) = self.base_highlight_index.get(&object_ref) else {
                continue;
            };
            let (y, inner_x0, inner_x1, fill_char) = match object_ref.category().segments() {
                [a, b] if a == "flow" && b == "node" => {
                    let Some((y0, x0, x1)) = hint_bounds_from_spans(spans) else {
                        continue;
                    };
                    let inner_x0 = x0.saturating_add(1);
                    let inner_x1 = x1.saturating_sub(1);
                    (y0.saturating_add(1), inner_x0, inner_x1, ' ')
                }
                [a, b] if a == "flow" && b == "edge" => {
                    let Some((y, inner_x0, inner_x1)) = flow_edge_hint_bounds(spans, &lines) else {
                        continue;
                    };
                    (y, inner_x0, inner_x1, crate::render::UNICODE_BOX_HORIZONTAL)
                }
                [a, b] if a == "seq" && b == "participant" => {
                    let Some((y0, x0, x1)) = hint_bounds_from_spans(spans) else {
                        continue;
                    };
                    let inner_x0 = x0.saturating_add(1);
                    let inner_x1 = x1.saturating_sub(1);
                    (y0.saturating_add(1), inner_x0, inner_x1, ' ')
                }
                [a, b] if a == "seq" && b == "message" => {
                    let Some((y0, x0, x1)) = hint_bounds_from_spans(spans) else {
                        continue;
                    };
                    let inner_x0 = x0.saturating_add(1);
                    let inner_x1 = x1.saturating_sub(1);
                    (y0, inner_x0, inner_x1, '─')
                }
                _ => continue,
            };
            if inner_x0 >= inner_x1 {
                continue;
            }

            let Some(line) = lines.get(y) else {
                continue;
            };
            if !is_flow_edge_ref(&object_ref)
                && !hint_range_has_text(line, inner_x0, inner_x1, fill_char)
            {
                continue;
            }

            placements.push((object_ref, (y, inner_x0, inner_x1, fill_char)));
        }

        placements.sort_by(|(left, _), (right, _)| right.to_string().cmp(&left.to_string()));

        let hint_count = placements.len();
        if hint_count == 0 {
            return None;
        }

        let k = NODE_HINT_CHARS.chars().count();
        let max_two_char = k.saturating_mul(k);
        if hint_count > max_two_char {
            self.set_toast(format!("Too many hint targets ({hint_count}, max {max_two_char})"));
            return None;
        }

        let labels = hints::gen_labels(hint_count + k, NODE_HINT_CHARS)
            .into_iter()
            .skip(k)
            .take(hint_count)
            .collect::<Vec<_>>();

        let mut targets = Vec::<HintTarget>::with_capacity(hint_count);
        for ((object_ref, (y, inner_x0, inner_x1, fill_char)), label) in
            placements.into_iter().zip(labels)
        {
            let mut chars = label.chars();
            let a = chars.next().unwrap_or('A');
            let b = chars.next().unwrap_or('A');
            targets.push(HintTarget {
                label: [a, b],
                object_ref,
                y,
                inner_x0,
                inner_x1,
                fill_char,
            });
        }

        Some(targets)
    }

    fn select_chain_object_ref(&mut self, object_ref: &ObjectRef) {
        let prev = self.hint_select_chain_prev.clone();

        let mut refs_to_select = Vec::new();
        refs_to_select.push(object_ref.clone());

        if let Some(prev) = prev.as_ref() {
            if let Some(connector) = self.chain_connector_ref(object_ref, prev) {
                refs_to_select.push(connector);
            }
        }

        let mut inserted_any = false;
        for object_ref in &refs_to_select {
            inserted_any |= self.session.selected_object_refs_mut().insert(object_ref.clone());
        }

        if inserted_any {
            self.apply_object_filters();
        }

        self.select_object_ref(object_ref);
        self.hint_select_chain_prev = Some(object_ref.clone());

        let mut message = match refs_to_select.as_slice() {
            [node] => format!("Selected {node}"),
            [node, connector] => format!("Selected {node} + {connector}"),
            _ => format!("Selected {}", object_ref),
        };

        if inserted_any {
            if let Some(session_folder) = self.session_folder.as_ref() {
                if let Err(err) = session_folder.save_selected_object_refs(&self.session) {
                    message = format!("{message} (persist failed: {err})");
                }
            }
        }
        self.set_toast(message);
    }

    fn chain_connector_ref(&self, current: &ObjectRef, previous: &ObjectRef) -> Option<ObjectRef> {
        if current == previous {
            return None;
        }
        if current.diagram_id() != previous.diagram_id() {
            return None;
        }
        let diagram = self.session.diagrams().get(current.diagram_id())?;

        match diagram.ast() {
            DiagramAst::Flowchart(ast) => {
                let is_flow_node = |r: &ObjectRef| matches!(r.category().segments(), [a, b] if a == "flow" && b == "node");
                if !is_flow_node(current) || !is_flow_node(previous) {
                    return None;
                }

                let current_id = current.object_id();
                let previous_id = previous.object_id();

                let mut edge_id = ast
                    .edges()
                    .iter()
                    .find(|(_, edge)| {
                        edge.from_node_id() == current_id && edge.to_node_id() == previous_id
                    })
                    .map(|(edge_id, _)| edge_id.clone());

                if edge_id.is_none() {
                    edge_id = ast
                        .edges()
                        .iter()
                        .find(|(_, edge)| {
                            edge.from_node_id() == previous_id && edge.to_node_id() == current_id
                        })
                        .map(|(edge_id, _)| edge_id.clone());
                }

                edge_id.map(|edge_id| {
                    ObjectRef::new(
                        current.diagram_id().clone(),
                        category_path(&["flow", "edge"]),
                        edge_id,
                    )
                })
            }
            DiagramAst::Sequence(ast) => {
                let is_seq_participant = |r: &ObjectRef| matches!(r.category().segments(), [a, b] if a == "seq" && b == "participant");
                if !is_seq_participant(current) || !is_seq_participant(previous) {
                    return None;
                }

                let current_id = current.object_id();
                let previous_id = previous.object_id();

                let messages = ast.messages_in_order();

                let mut message_id = messages
                    .iter()
                    .rev()
                    .find(|msg| {
                        msg.from_participant_id() == current_id
                            && msg.to_participant_id() == previous_id
                    })
                    .map(|msg| msg.message_id().clone());

                if message_id.is_none() {
                    message_id = messages
                        .iter()
                        .rev()
                        .find(|msg| {
                            msg.from_participant_id() == previous_id
                                && msg.to_participant_id() == current_id
                        })
                        .map(|msg| msg.message_id().clone());
                }

                message_id.map(|message_id| {
                    ObjectRef::new(
                        current.diagram_id().clone(),
                        category_path(&["seq", "message"]),
                        message_id,
                    )
                })
            }
        }
    }

    fn handle_objects_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('h') => self.select_prev(),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('l') => self.select_next(),

            KeyCode::Home => self.select_first(),
            KeyCode::End => self.select_last(),
            KeyCode::Char(' ') => self.toggle_selected_object(),
            KeyCode::Char('-') => self.toggle_objects_selected_only(),
            KeyCode::Char('y') => self.yank_selected_object_ref(),
            KeyCode::Char('f') => self.enter_diagram_hint_mode(),
            KeyCode::Char('c') => self.enter_diagram_select_hint_mode(),
            KeyCode::Char('g') => self.jump_to_selected_incoming_xref(),
            KeyCode::Char('t') => self.jump_to_selected_outgoing_xref(),

            _ => {}
        }
    }

    fn handle_xrefs_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('h') => self.select_xref_prev(),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('l') => self.select_xref_next(),

            KeyCode::Home => self.select_xref_first(),
            KeyCode::End => self.select_xref_last(),

            KeyCode::Char('-') => self.toggle_xrefs_dangling_only(),
            KeyCode::Char('I') => self.toggle_xrefs_involving_only(),
            KeyCode::Char('g') => self.jump_to_xref_from(),
            KeyCode::Char('t') => self.jump_to_xref_to(),

            _ => {}
        }
    }

    fn select_prev(&mut self) {
        let visible = self.visible_object_indices();
        let len = visible.len();
        if len == 0 {
            self.objects_state.select(None);
            self.publish_focus_to_ui_state();
            return;
        }

        let idx = self.objects_state.selected().unwrap_or(0).min(len - 1);
        let next = idx.saturating_sub(1);
        self.objects_state.select(Some(next));
        if self.xrefs_involving_only {
            self.apply_xref_filters();
        }
        self.publish_focus_to_ui_state();
    }

    fn select_next(&mut self) {
        let visible = self.visible_object_indices();
        let len = visible.len();
        if len == 0 {
            self.objects_state.select(None);
            self.publish_focus_to_ui_state();
            return;
        }

        let idx = self.objects_state.selected().unwrap_or(0).min(len - 1);
        let next = (idx + 1).min(len - 1);
        self.objects_state.select(Some(next));
        if self.xrefs_involving_only {
            self.apply_xref_filters();
        }
        self.publish_focus_to_ui_state();
    }

    fn select_first(&mut self) {
        if self.visible_object_indices().is_empty() {
            self.objects_state.select(None);
            self.publish_focus_to_ui_state();
            return;
        }
        self.objects_state.select(Some(0));
        if self.xrefs_involving_only {
            self.apply_xref_filters();
        }
        self.publish_focus_to_ui_state();
    }

    fn select_last(&mut self) {
        let visible = self.visible_object_indices();
        if visible.is_empty() {
            self.objects_state.select(None);
            self.publish_focus_to_ui_state();
            return;
        }
        self.objects_state.select(Some(visible.len() - 1));
        if self.xrefs_involving_only {
            self.apply_xref_filters();
        }
        self.publish_focus_to_ui_state();
    }

    fn toggle_selected_object(&mut self) {
        let Some(object_ref) = self.selected_ref().cloned() else {
            self.set_toast("No object selected");
            return;
        };

        let now_selected = if self.session.selected_object_refs_mut().remove(&object_ref) {
            false
        } else {
            self.session.selected_object_refs_mut().insert(object_ref.clone());
            true
        };

        self.apply_object_filters();

        let verb = if now_selected { "Selected" } else { "Deselected" };
        let mut message = format!("{verb} {object_ref}");
        if let Some(session_folder) = self.session_folder.as_ref() {
            if let Err(err) = session_folder.save_selected_object_refs(&self.session) {
                message = format!("{message} (persist failed: {err})");
            }
        }
        self.set_toast(message);
    }

    fn deselect_current_diagram_objects(&mut self) {
        let Some(active_diagram_id) = self.active_diagram_id().cloned() else {
            return;
        };

        let before = self.session.selected_object_refs().len();
        self.session
            .selected_object_refs_mut()
            .retain(|object_ref| object_ref.diagram_id() != &active_diagram_id);
        let removed = before.saturating_sub(self.session.selected_object_refs().len());

        if removed == 0 {
            self.set_toast("No selected objects in current diagram");
            return;
        }

        self.apply_object_filters();

        let mut message = format!("Deselected {removed} object(s) in {active_diagram_id}");
        if let Some(session_folder) = self.session_folder.as_ref() {
            if let Err(err) = session_folder.save_selected_object_refs(&self.session) {
                message = format!("{message} (persist failed: {err})");
            }
        }
        self.set_toast(message);
    }

    fn toggle_objects_selected_only(&mut self) {
        self.objects_selected_only = !self.objects_selected_only;
        self.apply_object_filters();
        self.set_toast(if self.objects_selected_only {
            "Showing selected objects"
        } else {
            "Showing all objects"
        });
    }

    fn select_xref_prev(&mut self) {
        let visible = self.visible_xref_indices();
        let len = visible.len();
        if len == 0 {
            self.xrefs_state.select(None);
            return;
        }

        let idx = self.xrefs_state.selected().unwrap_or(0);
        let next = idx.saturating_sub(1);
        self.xrefs_state.select(Some(next));
    }

    fn select_xref_next(&mut self) {
        let visible = self.visible_xref_indices();
        let len = visible.len();
        if len == 0 {
            self.xrefs_state.select(None);
            return;
        }

        let idx = self.xrefs_state.selected().unwrap_or(0);
        let next = (idx + 1).min(len - 1);
        self.xrefs_state.select(Some(next));
    }

    fn select_xref_first(&mut self) {
        if self.visible_xref_indices().is_empty() {
            self.xrefs_state.select(None);
            return;
        }
        self.xrefs_state.select(Some(0));
    }

    fn select_xref_last(&mut self) {
        let visible = self.visible_xref_indices();
        if visible.is_empty() {
            self.xrefs_state.select(None);
            return;
        }
        self.xrefs_state.select(Some(visible.len() - 1));
    }

    fn jump_to_object_ref(&mut self, object_ref: &ObjectRef) {
        self.select_object_ref(object_ref);
    }

    fn select_object_ref(&mut self, object_ref: &ObjectRef) {
        if self.session.diagrams().get(object_ref.diagram_id()).is_none() {
            return;
        }

        let active_matches = self
            .session
            .active_diagram_id()
            .map(|active| active == object_ref.diagram_id())
            .unwrap_or(false);
        if !active_matches {
            self.set_active_diagram_id(object_ref.diagram_id().clone());
        }

        let Some(object_idx) = self.object_index_for_ref(object_ref) else {
            return;
        };

        if self.objects_selected_only && !self.session.selected_object_refs().contains(object_ref) {
            self.objects_selected_only = false;
            self.apply_object_filters();
        }

        let Some(visible_idx) =
            self.visible_object_indices.iter().position(|&idx| idx == object_idx)
        else {
            return;
        };
        self.objects_state.select(Some(visible_idx));
        if self.xrefs_involving_only {
            self.apply_xref_filters();
        }
        self.publish_focus_to_ui_state();
    }

    fn jump_to_xref_from(&mut self) {
        let Some(idx) = self.selected_xref_index() else {
            return;
        };
        let object_ref = self.xrefs[idx].xref.from().clone();
        self.jump_to_object_ref(&object_ref);
    }

    fn jump_to_xref_to(&mut self) {
        let Some(idx) = self.selected_xref_index() else {
            return;
        };
        let object_ref = self.xrefs[idx].xref.to().clone();
        self.jump_to_object_ref(&object_ref);
    }

    fn jump_to_selected_outgoing_xref(&mut self) {
        let Some(selected_ref) = self.selected_ref().cloned() else {
            self.set_toast("No object selected");
            return;
        };

        let matches = self
            .xrefs
            .iter()
            .enumerate()
            .filter(|(_, xref)| xref.xref.from() == &selected_ref)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        let Some(first_idx) = matches.first().copied() else {
            self.set_toast(format!("No outgoing xref for {selected_ref}"));
            return;
        };
        let target_ref = self.xrefs[first_idx].xref.to().clone();
        self.jump_to_object_ref(&target_ref);

        if matches.len() > 1 {
            let xref_id = &self.xrefs[first_idx].xref_id;
            self.set_toast(format!("{} outgoing xrefs; followed first ({xref_id})", matches.len()));
        }
    }

    fn jump_to_selected_incoming_xref(&mut self) {
        let Some(selected_ref) = self.selected_ref().cloned() else {
            self.set_toast("No object selected");
            return;
        };

        let matches = self
            .xrefs
            .iter()
            .enumerate()
            .filter(|(_, xref)| xref.xref.to() == &selected_ref)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        let Some(first_idx) = matches.first().copied() else {
            self.set_toast(format!("No incoming xref for {selected_ref}"));
            return;
        };
        let source_ref = self.xrefs[first_idx].xref.from().clone();
        self.jump_to_object_ref(&source_ref);

        if matches.len() > 1 {
            let xref_id = &self.xrefs[first_idx].xref_id;
            self.set_toast(format!("{} incoming xrefs; followed first ({xref_id})", matches.len()));
        }
    }

    fn set_toast(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast {
            message: message.into(),
            expires_at: Instant::now() + Duration::from_secs(2),
        });
    }

    fn yank_selected_object_ref(&mut self) {
        let Some(object_ref) = self.selected_ref() else {
            self.set_toast("No object selected");
            return;
        };

        let object_ref = object_ref.to_string();
        match copy_to_clipboard(&object_ref) {
            Ok(backend) => {
                self.set_toast(format!("Yanked object ref ({backend})"));
            }
            Err(err) => {
                self.set_toast(format!("Clipboard error: {err}"));
            }
        }
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalSession {
    fn new() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|err| {
            teardown_terminal();
            err
        })?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|err| {
            teardown_terminal();
            err
        })?;
        terminal.clear().map_err(|err| {
            teardown_terminal();
            err
        })?;

        Ok(Self { terminal })
    }

    fn draw(&mut self, draw_fn: impl FnOnce(&mut Frame<'_>)) -> io::Result<()> {
        self.terminal.draw(draw_fn)?;
        Ok(())
    }

    fn run_external_action(
        &mut self,
        action: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let _suspend = TerminalSuspendGuard::new(&mut self.terminal)
            .map_err(|err| format!("terminal suspend failed: {err}"))?;
        action()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        teardown_terminal();
    }
}

struct TerminalSuspendGuard<'a> {
    terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>,
}

impl<'a> TerminalSuspendGuard<'a> {
    fn new(terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<Self> {
        terminal.show_cursor()?;
        disable_raw_mode()?;

        if let Err(err) = execute!(terminal.backend_mut(), LeaveAlternateScreen) {
            let _ = enable_raw_mode();
            let _ = execute!(terminal.backend_mut(), EnterAlternateScreen);
            let _ = terminal.hide_cursor();
            let _ = ratatui::backend::Backend::flush(terminal.backend_mut());
            return Err(err);
        }

        ratatui::backend::Backend::flush(terminal.backend_mut())?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalSuspendGuard<'_> {
    fn drop(&mut self) {
        let _ = enable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), EnterAlternateScreen);
        let _ = self.terminal.clear();
        let _ = self.terminal.hide_cursor();
        let _ = ratatui::backend::Backend::flush(self.terminal.backend_mut());
    }
}

fn teardown_terminal() {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen);
}

fn copy_to_clipboard(text: &str) -> Result<&'static str, String> {
    let mut stdout = io::stdout();
    execute!(stdout, Print(osc52_sequence(text))).map_err(|err| err.to_string())?;
    Ok("osc52")
}

fn osc52_sequence(text: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;

    let encoded = STANDARD.encode(text.as_bytes());
    format!("\x1b]52;c;{encoded}\x1b\\")
}

fn resolve_editor_command() -> String {
    env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env::var("EDITOR").ok().filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| "vi".to_owned())
}

fn write_temp_mermaid_file(
    diagram_id: &DiagramId,
    content: &str,
) -> Result<std::path::PathBuf, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let mut temp_path = env::temp_dir();
    let safe_id = diagram_id
        .to_string()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' })
        .collect::<String>();
    temp_path.push(format!("nereid-{safe_id}-{ts}.mmd"));
    fs::write(&temp_path, content).map_err(|err| {
        format!("failed to create temporary Mermaid file {}: {err}", temp_path.display())
    })?;
    Ok(temp_path)
}

fn launch_editor_command(command: &str, path: &Path) -> Result<(), String> {
    let path_text = path.to_string_lossy();
    if path_text.starts_with('-') {
        return Err("invalid editor temp path".to_owned());
    }

    let status = Command::new("sh")
        .arg("-lc")
        .arg(format!("{command} {}", shell_single_quote(path_text.as_ref())))
        .status()
        .map_err(|err| format!("failed to run editor command `{command}`: {err}"))?;
    if !status.success() {
        return Err(format!("editor command failed with status {status}"));
    }
    Ok(())
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn export_diagram_mermaid(diagram: &Diagram) -> Result<String, String> {
    match diagram.ast() {
        DiagramAst::Sequence(ast) => export_sequence_diagram(ast)
            .map_err(|err| format!("failed to export sequence Mermaid: {err}")),
        DiagramAst::Flowchart(ast) => export_flowchart(ast)
            .map_err(|err| format!("failed to export flowchart Mermaid: {err}")),
    }
}

fn parse_mermaid_for_kind(kind: DiagramKind, source: &str) -> Result<DiagramAst, String> {
    match kind {
        DiagramKind::Sequence => parse_sequence_diagram(source)
            .map(DiagramAst::Sequence)
            .map_err(|err| format!("sequence parse failed: {err}")),
        DiagramKind::Flowchart => parse_flowchart(source)
            .map(DiagramAst::Flowchart)
            .map_err(|err| format!("flowchart parse failed: {err}")),
    }
}

fn ensure_active_diagram_id(session: &mut Session) -> Option<DiagramId> {
    if let Some(active) = session.active_diagram_id().cloned() {
        return Some(active);
    }

    let first = session.diagrams().keys().next().cloned()?;

    session.set_active_diagram_id(Some(first.clone()));
    Some(first)
}

fn render_diagram_annotated(diagram: &Diagram, options: RenderOptions) -> (String, HighlightIndex) {
    match crate::render::diagram::render_diagram_unicode_annotated_with_options(diagram, options) {
        Ok(rendered) => (rendered.text, rendered.highlight_index),
        Err(err) => (format!("Diagram render error:\n{err}"), HighlightIndex::new()),
    }
}

fn strip_direction_prefix(label: &str) -> &str {
    for prefix in ["▾▴ ", "▾ ", "▴ ", "▾  ", " ▴ "] {
        if let Some(stripped) = label.strip_prefix(prefix) {
            return stripped;
        }
    }
    label
}

fn prefixed_direction_label(label: &str, has_incoming: bool, has_outgoing: bool) -> String {
    let stripped = strip_direction_prefix(label);
    let prefix = xref_direction_prefix_for_flags(has_outgoing, has_incoming);
    if prefix.is_empty() {
        stripped.to_owned()
    } else {
        format!("{prefix}{stripped}")
    }
}

fn prefix_xref_direction_labels_for_tui(diagram: &mut Diagram, session: &Session) {
    let mut incoming_refs = BTreeSet::<ObjectRef>::new();
    let mut outgoing_refs = BTreeSet::<ObjectRef>::new();
    for xref in session.xrefs().values() {
        if xref.to().diagram_id() == diagram.diagram_id() {
            incoming_refs.insert(xref.to().clone());
        }
        if xref.from().diagram_id() == diagram.diagram_id() {
            outgoing_refs.insert(xref.from().clone());
        }
    }
    if incoming_refs.is_empty() && outgoing_refs.is_empty() {
        return;
    }

    let mut ast = diagram.ast().clone();
    match &mut ast {
        DiagramAst::Flowchart(flow_ast) => {
            let node_category = category_path(&["flow", "node"]);
            for (node_id, node) in flow_ast.nodes_mut() {
                let object_ref = ObjectRef::new(
                    diagram.diagram_id().clone(),
                    node_category.clone(),
                    node_id.clone(),
                );
                let has_incoming = incoming_refs.contains(&object_ref);
                let has_outgoing = outgoing_refs.contains(&object_ref);
                node.set_label(prefixed_direction_label(node.label(), has_incoming, has_outgoing));
            }
        }
        DiagramAst::Sequence(seq_ast) => {
            let participant_category = category_path(&["seq", "participant"]);
            for (participant_id, participant) in seq_ast.participants_mut() {
                let object_ref = ObjectRef::new(
                    diagram.diagram_id().clone(),
                    participant_category.clone(),
                    participant_id.clone(),
                );
                let has_incoming = incoming_refs.contains(&object_ref);
                let has_outgoing = outgoing_refs.contains(&object_ref);
                participant.set_mermaid_name(prefixed_direction_label(
                    participant.mermaid_name(),
                    has_incoming,
                    has_outgoing,
                ));
            }

            let message_category = category_path(&["seq", "message"]);
            for message in seq_ast.messages_mut() {
                let object_ref = ObjectRef::new(
                    diagram.diagram_id().clone(),
                    message_category.clone(),
                    message.message_id().clone(),
                );
                let mut updated = SequenceMessage::new(
                    message.message_id().clone(),
                    message.from_participant_id().clone(),
                    message.to_participant_id().clone(),
                    message.kind(),
                    prefixed_direction_label(
                        message.text(),
                        incoming_refs.contains(&object_ref),
                        outgoing_refs.contains(&object_ref),
                    ),
                    message.order_key(),
                );
                updated.set_raw_arrow(message.raw_arrow().map(str::to_owned));
                *message = updated;
            }
        }
    }

    diagram.set_ast(ast).expect("diagram kind should remain unchanged");
}

fn render_diagram_annotated_for_tui(
    session: &Session,
    diagram: &Diagram,
    show_notes: bool,
) -> (String, HighlightIndex) {
    let mut render_diagram = diagram.clone();
    prefix_xref_direction_labels_for_tui(&mut render_diagram, session);
    render_diagram_annotated(
        &render_diagram,
        RenderOptions {
            show_notes,
            prefix_object_labels: false,
            flowchart_extra_col_gap: TUI_FLOWCHART_EXTRA_COL_GAP,
        },
    )
}

fn apply_highlight_flags(flags_by_line: &mut [Vec<u8>], spans: &[LineSpan], flag: u8) {
    for (y, x0, x1) in spans {
        let Some(line) = flags_by_line.get_mut(*y) else {
            continue;
        };
        if line.is_empty() {
            continue;
        }

        let max_x = line.len().saturating_sub(1);
        let start = (*x0).min(max_x);
        let end = (*x1).min(max_x);
        for cell in line.iter_mut().take(end + 1).skip(start) {
            *cell |= flag;
        }
    }
}

fn is_box_drawing_verticalish(ch: char) -> bool {
    matches!(ch, '│' | '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼')
}

fn is_box_drawing_horizontalish(ch: char) -> bool {
    matches!(ch, '─' | '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼')
}

fn fill_highlight_bridge_gaps(flags_by_line: &mut [Vec<u8>], diagram: &str, flag: u8) {
    // Fill only a single hidden crossing cell to avoid over-highlighting through dense bundles.
    fill_highlight_bridge_gaps_with_limit(flags_by_line, diagram, flag, 1);
}

fn fill_highlight_bridge_gaps_unbounded(flags_by_line: &mut [Vec<u8>], diagram: &str, flag: u8) {
    fill_highlight_bridge_gaps_with_limit(flags_by_line, diagram, flag, usize::MAX);
}

fn fill_highlight_text_space_gaps(flags_by_line: &mut [Vec<u8>], diagram: &str, flag: u8) {
    const MAX_TEXT_SPACE_GAP: usize = 3;

    for (y, line) in diagram.split('\n').enumerate() {
        let Some(flags) = flags_by_line.get_mut(y) else {
            continue;
        };
        if flags.len() < 3 {
            continue;
        }

        let chars = line.chars().collect::<Vec<_>>();
        let len = flags.len().min(chars.len());
        if len < 3 {
            continue;
        }

        let mut idx = 0usize;
        while idx < len {
            while idx < len && flags[idx] & flag != 0 {
                idx += 1;
            }
            let gap_start = idx;
            while idx < len && flags[idx] & flag == 0 {
                idx += 1;
            }
            let gap_end = idx;

            let gap_len = gap_end.saturating_sub(gap_start);
            if gap_len == 0 || gap_len > MAX_TEXT_SPACE_GAP {
                continue;
            }
            if gap_start == 0 || gap_end >= len {
                continue;
            }

            let left = gap_start.saturating_sub(1);
            let right = gap_end;
            if flags[left] & flag == 0 || flags[right] & flag == 0 {
                continue;
            }
            if chars[gap_start..gap_end].iter().any(|ch| *ch != ' ') {
                continue;
            }
            if chars[left] == ' ' || chars[right] == ' ' {
                continue;
            }
            if is_box_drawing_char(chars[left]) || is_box_drawing_char(chars[right]) {
                continue;
            }

            for cell in flags.iter_mut().take(gap_end).skip(gap_start) {
                *cell |= flag;
            }
        }
    }
}

fn fill_highlight_bridge_gaps_with_limit(
    flags_by_line: &mut [Vec<u8>],
    diagram: &str,
    flag: u8,
    max_gap: usize,
) {
    for (y, line) in diagram.split('\n').enumerate() {
        let Some(flags) = flags_by_line.get_mut(y) else {
            continue;
        };
        if flags.is_empty() {
            continue;
        }

        let chars = line.chars().collect::<Vec<_>>();
        let len = flags.len().min(chars.len());
        if len < 3 {
            continue;
        }

        let mut idx = 0usize;
        while idx < len {
            while idx < len && flags[idx] & flag == 0 {
                idx += 1;
            }
            while idx < len && flags[idx] & flag != 0 {
                idx += 1;
            }

            let gap_start = idx;
            while idx < len && flags[idx] & flag == 0 {
                idx += 1;
            }

            if idx >= len {
                break;
            }

            let gap_end = idx;
            let gap_len = gap_end.saturating_sub(gap_start);
            if gap_len == 0 || gap_len > max_gap {
                continue;
            }

            if chars[gap_start..gap_end].iter().all(|ch| is_box_drawing_verticalish(*ch)) {
                for cell in flags.iter_mut().take(gap_end).skip(gap_start) {
                    *cell |= flag;
                }
            }
        }
    }
}

fn is_flow_edge_ref(object_ref: &ObjectRef) -> bool {
    matches!(
        object_ref.category().segments(),
        [a, b] if a == "flow" && b == "edge"
    )
}

fn is_sequence_block_or_section_ref(object_ref: &ObjectRef) -> bool {
    matches!(
        object_ref.category().segments(),
        [a, b] if a == "seq" && (b == "block" || b == "section")
    )
}

fn is_sequence_section_ref(object_ref: &ObjectRef) -> bool {
    matches!(object_ref.category().segments(), [a, b] if a == "seq" && b == "section")
}

fn is_note_ref(object_ref: &ObjectRef) -> bool {
    matches!(
        object_ref.category().segments(),
        [a, b] if (a == "seq" || a == "flow") && b == "note"
    )
}

fn apply_presence_flags(flags_by_line: &mut [Vec<bool>], spans: &[LineSpan]) {
    for (y, x0, x1) in spans {
        let Some(line) = flags_by_line.get_mut(*y) else {
            continue;
        };
        if line.is_empty() {
            continue;
        }

        let start = (*x0).min(line.len().saturating_sub(1));
        let end = (*x1).min(line.len().saturating_sub(1));
        for cell in line.iter_mut().take(end + 1).skip(start) {
            *cell = true;
        }
    }
}

fn apply_bounding_area_flags(flags_by_line: &mut [Vec<bool>], spans: &[LineSpan]) {
    let Some((min_y, min_x, max_y, max_x)) = bounding_box_from_spans(spans) else {
        return;
    };

    for y in min_y..=max_y {
        let Some(line) = flags_by_line.get_mut(y) else {
            continue;
        };
        if line.is_empty() {
            continue;
        }
        let start = min_x.min(line.len().saturating_sub(1));
        let end = max_x.min(line.len().saturating_sub(1));
        for cell in line.iter_mut().take(end + 1).skip(start) {
            *cell = true;
        }
    }
}

fn bounding_box_from_spans(spans: &[LineSpan]) -> Option<(usize, usize, usize, usize)> {
    let mut min_y = usize::MAX;
    let mut min_x = usize::MAX;
    let mut max_y = 0usize;
    let mut max_x = 0usize;
    let mut has_any = false;

    for (y, x0, x1) in spans {
        has_any = true;
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
        min_x = min_x.min((*x0).min(*x1));
        max_x = max_x.max((*x0).max(*x1));
    }

    has_any.then_some((min_y, min_x, max_y, max_x))
}

fn fill_highlight_corner_branch_extensions(flags_by_line: &mut [Vec<u8>], diagram: &str, flag: u8) {
    let chars_by_line =
        diagram.split('\n').map(|line| line.chars().collect::<Vec<_>>()).collect::<Vec<_>>();

    let mut extensions = Vec::<(usize, usize)>::new();

    for (y, chars) in chars_by_line.iter().enumerate() {
        let len = chars.len();
        let Some(flags) = flags_by_line.get(y) else {
            continue;
        };
        let width = flags.len().min(len);
        if width < 3 {
            continue;
        }

        for x in 1..(width - 1) {
            if flags[x] & flag == 0 {
                continue;
            }

            let has_up = y > 0
                && flags_by_line
                    .get(y - 1)
                    .and_then(|line| line.get(x))
                    .is_some_and(|cell| cell & flag != 0);
            let has_down = flags_by_line
                .get(y + 1)
                .and_then(|line| line.get(x))
                .is_some_and(|cell| cell & flag != 0);
            if !has_up && !has_down {
                continue;
            }

            let has_left = flags[x - 1] & flag != 0;
            let has_right = flags[x + 1] & flag != 0;
            if has_left == has_right {
                continue;
            }

            let extension_x = if has_left { x + 1 } else { x - 1 };
            if extension_x >= width {
                continue;
            }
            if flags[extension_x] & flag != 0 {
                continue;
            }
            if !is_box_drawing_horizontalish(chars[extension_x]) {
                continue;
            }

            extensions.push((y, extension_x));
        }
    }

    for (y, x) in extensions {
        if let Some(flags) = flags_by_line.get_mut(y) {
            if let Some(cell) = flags.get_mut(x) {
                *cell |= flag;
            }
        }
    }
}

fn hint_bounds_from_spans(spans: &[LineSpan]) -> Option<(usize, usize, usize)> {
    let mut min_y = None::<usize>;
    let mut min_x0 = None::<usize>;
    let mut max_x1 = None::<usize>;

    for (y, x0, x1) in spans {
        min_y = Some(min_y.map_or(*y, |current| current.min(*y)));
        min_x0 = Some(min_x0.map_or(*x0, |current| current.min(*x0)));
        max_x1 = Some(max_x1.map_or(*x1, |current| current.max(*x1)));
    }

    Some((min_y?, min_x0?, max_x1?))
}

fn flow_edge_hint_bounds(spans: &[LineSpan], lines: &[&str]) -> Option<(usize, usize, usize)> {
    let mut best = None::<(i64, usize, usize, usize, usize)>;

    for (y, x0, x1) in spans {
        let Some(line) = lines.get(*y) else {
            continue;
        };
        let chars = line.chars().collect::<Vec<_>>();
        if chars.is_empty() {
            continue;
        }

        let max_x = chars.len().saturating_sub(1);
        let mut start = (*x0).min(max_x);
        let mut end = (*x1).min(max_x);
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }

        let mut first = None::<usize>;
        let mut last = None::<usize>;
        for x in start..=end {
            if is_flow_edge_canvas_char(chars[x]) {
                first = Some(first.unwrap_or(x));
                last = Some(x);
            }
        }
        if let (Some(first), Some(last)) = (first, last) {
            start = first;
            end = last;
        }

        if end.saturating_sub(start).saturating_add(1) < 3 {
            end = (start + 2).min(max_x);
            start = end.saturating_sub(2);
        }
        if end.saturating_sub(start).saturating_add(1) < 3 {
            continue;
        }

        let width = end.saturating_sub(start).saturating_add(1);
        let edge_cells = (start..=end).filter(|&x| is_flow_edge_canvas_char(chars[x])).count();
        let text_cells = (start..=end).filter(|&x| is_hint_label_char(chars[x], ' ')).count();
        let score = (edge_cells as i64) * 10 + (width.min(9) as i64) - (text_cells as i64) * 12;
        let candidate = (score, width, *y, start, end);

        match best {
            Some((best_score, best_width, best_y, best_start, _))
                if best_score > score
                    || (best_score == score && best_width > width)
                    || (best_score == score && best_width == width && best_y > *y)
                    || (best_score == score
                        && best_width == width
                        && best_y == *y
                        && best_start <= start) => {}
            _ => best = Some(candidate),
        }
    }

    best.map(|(_, _, y, start, end)| (y, start, end))
}

fn is_flow_edge_canvas_char(ch: char) -> bool {
    is_box_drawing_char(ch) || matches!(ch, '▶' | '◀' | '▲' | '▼' | '○' | '✕')
}

fn is_box_drawing_char(ch: char) -> bool {
    matches!(
        ch,
        crate::render::UNICODE_BOX_HORIZONTAL
            | crate::render::UNICODE_BOX_VERTICAL
            | crate::render::UNICODE_BOX_TOP_LEFT
            | crate::render::UNICODE_BOX_TOP_RIGHT
            | crate::render::UNICODE_BOX_BOTTOM_LEFT
            | crate::render::UNICODE_BOX_BOTTOM_RIGHT
            | crate::render::UNICODE_BOX_TEE_RIGHT
            | crate::render::UNICODE_BOX_TEE_LEFT
            | crate::render::UNICODE_BOX_TEE_DOWN
            | crate::render::UNICODE_BOX_TEE_UP
            | crate::render::UNICODE_BOX_CROSS
    )
}

fn is_flow_edge_hint_slot_char(ch: char) -> bool {
    ch == ' ' || is_flow_edge_canvas_char(ch)
}

fn is_hint_label_char(ch: char, fill_char: char) -> bool {
    ch != fill_char && !is_box_drawing_char(ch)
}

fn hint_range_has_text(line: &str, x0: usize, x1: usize, fill_char: char) -> bool {
    if x0 > x1 {
        return false;
    }

    for (idx, ch) in line.chars().enumerate() {
        if idx > x1 {
            break;
        }
        if idx >= x0 && is_hint_label_char(ch, fill_char) {
            return true;
        }
    }

    false
}

fn apply_hint_target_to_line(
    line_chars: &mut Vec<char>,
    flags: &mut Vec<u8>,
    style_overrides: &mut Vec<Option<Style>>,
    target: &HintTarget,
    typed_first: Option<char>,
) {
    if target.inner_x0 > target.inner_x1 {
        return;
    }

    let needed_len = target.inner_x1.saturating_add(1);
    if line_chars.len() < needed_len {
        line_chars.resize(needed_len, target.fill_char);
        flags.resize(needed_len, 0);
        style_overrides.resize(needed_len, None);
    }

    let base_style = Style::default().fg(Color::White).bg(Color::Cyan).add_modifier(Modifier::BOLD);
    let typed_style =
        Style::default().fg(Color::DarkGray).bg(Color::Cyan).add_modifier(Modifier::BOLD);

    if is_flow_edge_ref(&target.object_ref) {
        if target.inner_x0 >= target.inner_x1 {
            return;
        }

        let max_start = target.inner_x1.saturating_sub(1);
        let mut tag_start = None::<usize>;
        for x in target.inner_x0..=max_start {
            let ch0 = line_chars.get(x).copied().unwrap_or(target.fill_char);
            let ch1 = line_chars.get(x.saturating_add(1)).copied().unwrap_or(target.fill_char);
            let free_slot = style_overrides.get(x).and_then(|style| *style).is_none()
                && style_overrides.get(x.saturating_add(1)).and_then(|style| *style).is_none();
            if free_slot && is_flow_edge_hint_slot_char(ch0) && is_flow_edge_hint_slot_char(ch1) {
                tag_start = Some(x);
                break;
            }
        }

        if tag_start.is_none() {
            let max_pair_start = line_chars.len().saturating_sub(2);
            let search_start = target.inner_x0.saturating_sub(4).min(max_pair_start);
            let search_end = target.inner_x1.saturating_add(4).min(max_pair_start);
            if search_start <= search_end {
                for x in search_start..=search_end {
                    let ch0 = line_chars.get(x).copied().unwrap_or(target.fill_char);
                    let ch1 =
                        line_chars.get(x.saturating_add(1)).copied().unwrap_or(target.fill_char);
                    let free_slot = style_overrides.get(x).and_then(|style| *style).is_none()
                        && style_overrides
                            .get(x.saturating_add(1))
                            .and_then(|style| *style)
                            .is_none();
                    if free_slot
                        && is_flow_edge_hint_slot_char(ch0)
                        && is_flow_edge_hint_slot_char(ch1)
                    {
                        tag_start = Some(x);
                        break;
                    }
                }
            }
        }

        let Some(tag_x0) = tag_start else {
            return;
        };
        let tag_x1 = tag_x0.saturating_add(1);
        if tag_x1 >= line_chars.len() {
            return;
        }

        line_chars[tag_x0] = target.label[0];
        line_chars[tag_x1] = target.label[1];
        if typed_first.is_some_and(|first| first == target.label[0]) {
            style_overrides[tag_x0] = Some(typed_style);
            style_overrides[tag_x1] = Some(base_style);
        } else {
            style_overrides[tag_x0] = Some(base_style);
            style_overrides[tag_x1] = Some(base_style);
        }
        return;
    }

    let mut label_start = None::<usize>;
    let mut label_end = None::<usize>;
    for x in target.inner_x0..=target.inner_x1 {
        let ch = line_chars.get(x).copied().unwrap_or(target.fill_char);
        if is_hint_label_char(ch, target.fill_char) {
            if label_start.is_none() {
                label_start = Some(x);
            }
            label_end = Some(x);
        }
    }

    let label_bounds = match (label_start, label_end) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    };

    if target.fill_char == crate::render::UNICODE_BOX_HORIZONTAL {
        let mut label_end = target.inner_x0.saturating_sub(1);

        if let Some((mut label_start, mut existing_label_end)) = label_bounds {
            let label_chars = line_chars[label_start..=existing_label_end].to_vec();
            let label_len = label_chars.len();
            if label_len == 0 {
                return;
            }

            let needed_space = 3usize;
            let mut right_space = target.inner_x1 - existing_label_end;
            if right_space < needed_space {
                let required_shift = needed_space - right_space;
                let left_pad = label_start.saturating_sub(target.inner_x0);
                let shift_left = required_shift.min(left_pad);

                if shift_left > 0 {
                    for x in label_start..=existing_label_end {
                        line_chars[x] = target.fill_char;
                        style_overrides[x] = None;
                    }

                    let new_label_start = label_start.saturating_sub(shift_left);
                    let new_label_end = new_label_start.saturating_add(label_len.saturating_sub(1));
                    for (offset, ch) in label_chars.iter().copied().enumerate() {
                        line_chars[new_label_start + offset] = ch;
                    }

                    label_start = new_label_start;
                    existing_label_end = new_label_end;
                }

                right_space = target.inner_x1 - existing_label_end;
                if right_space < needed_space {
                    let truncate = needed_space - right_space;
                    if truncate >= label_len {
                        for x in label_start..=existing_label_end {
                            line_chars[x] = target.fill_char;
                            style_overrides[x] = None;
                        }
                        existing_label_end = target.inner_x0.saturating_sub(1);
                    } else {
                        let start = (existing_label_end + 1).saturating_sub(truncate);
                        for x in start..=existing_label_end {
                            line_chars[x] = target.fill_char;
                            style_overrides[x] = None;
                        }
                        existing_label_end = existing_label_end.saturating_sub(truncate);
                    }
                }
            }

            label_end = existing_label_end;
        }

        let scan_start = label_end.saturating_add(1).max(target.inner_x0);
        let max_start = target.inner_x1.saturating_sub(2);
        if scan_start > max_start {
            return;
        }

        let mut tag_slot_start = None::<usize>;
        for x in scan_start..=max_start {
            if line_chars.get(x) == Some(&target.fill_char)
                && line_chars.get(x.saturating_add(1)) == Some(&target.fill_char)
                && line_chars.get(x.saturating_add(2)) == Some(&target.fill_char)
            {
                tag_slot_start = Some(x);
                break;
            }
        }

        if tag_slot_start.is_none() {
            for x in scan_start..=max_start {
                let ch0 = line_chars.get(x).copied().unwrap_or(target.fill_char);
                let ch1 = line_chars.get(x.saturating_add(1)).copied().unwrap_or(target.fill_char);
                let ch2 = line_chars.get(x.saturating_add(2)).copied().unwrap_or(target.fill_char);
                if !is_hint_label_char(ch0, target.fill_char)
                    && !is_hint_label_char(ch1, target.fill_char)
                    && !is_hint_label_char(ch2, target.fill_char)
                {
                    tag_slot_start = Some(x);
                    break;
                }
            }
        }

        let Some(tag_slot_start) = tag_slot_start else {
            return;
        };
        let space_x = tag_slot_start;
        let tag_x0 = space_x.saturating_add(1);
        let tag_x1 = tag_x0.saturating_add(1);
        if tag_x1 > target.inner_x1 {
            return;
        }

        line_chars[space_x] = ' ';
        style_overrides[space_x] = None;
        line_chars[tag_x0] = target.label[0];
        line_chars[tag_x1] = target.label[1];

        if typed_first.is_some_and(|first| first == target.label[0]) {
            style_overrides[tag_x0] = Some(typed_style);
            style_overrides[tag_x1] = Some(base_style);
        } else {
            style_overrides[tag_x0] = Some(base_style);
            style_overrides[tag_x1] = Some(base_style);
        }

        return;
    }

    let (label_start, label_end) = match label_bounds {
        Some((start, end)) => (start, end),
        None => return,
    };

    let mut label_chars = line_chars[label_start..=label_end].to_vec();
    if label_chars.is_empty() {
        return;
    }

    let available = target.inner_x1.saturating_sub(target.inner_x0).saturating_add(1);
    if available < 3 {
        return;
    }

    let right_space = target.inner_x1 - label_end;
    let needed_space = 3usize;
    let required_shift = needed_space.saturating_sub(right_space);
    let left_pad = label_start.saturating_sub(target.inner_x0);
    let shift_left = required_shift.min(left_pad);

    let new_label_start = label_start.saturating_sub(shift_left);
    let space_x = target.inner_x1.saturating_sub(2);
    let max_label_end = space_x.saturating_sub(1);
    let max_label_len = if new_label_start > max_label_end {
        0
    } else {
        max_label_end.saturating_sub(new_label_start).saturating_add(1)
    };
    label_chars.truncate(max_label_len);

    let tag_x0 = space_x.saturating_add(1);
    let tag_x1 = tag_x0.saturating_add(1);
    if tag_x1 > target.inner_x1 {
        return;
    }

    for x in target.inner_x0..=target.inner_x1 {
        line_chars[x] = target.fill_char;
        style_overrides[x] = None;
    }

    for (offset, ch) in label_chars.into_iter().enumerate() {
        line_chars[new_label_start + offset] = ch;
    }

    line_chars[space_x] = ' ';
    style_overrides[space_x] = None;
    line_chars[tag_x0] = target.label[0];
    line_chars[tag_x1] = target.label[1];

    if typed_first.is_some_and(|first| first == target.label[0]) {
        style_overrides[tag_x0] = Some(typed_style);
        style_overrides[tag_x1] = Some(base_style);
    } else {
        style_overrides[tag_x0] = Some(base_style);
        style_overrides[tag_x1] = Some(base_style);
    }
}

fn style_for_highlight_flag(
    flag: u8,
    has_active_selection_in_diagram: bool,
    focus_owner: FocusOwner,
) -> Style {
    let selected = flag & 0b100 != 0;
    let background_flags = flag & 0b011;
    let in_focus = background_flags != 0;

    let base = Style::default().add_modifier(Modifier::BOLD);
    let focus_bg = focus_color_for_owner(focus_owner);
    let mut style = match background_flags {
        0b01 => base.fg(Color::White).bg(focus_bg),
        0b10 => base.fg(Color::White).bg(AGENT_FOCUS_COLOR),
        0b11 => base.fg(Color::White).bg(AGENT_FOCUS_COLOR),
        _ => Style::default(),
    };

    if selected {
        style = if in_focus {
            Style::default().fg(Color::White).bg(Color::LightGreen).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        };
    } else if background_flags == 0 && has_active_selection_in_diagram {
        style = Style::default().fg(Color::DarkGray);
    }

    style
}

#[allow(clippy::too_many_arguments)]
fn style_for_diagram_cell(
    flag: u8,
    has_active_selection_in_diagram: bool,
    focus_owner: FocusOwner,
    is_note_cell: bool,
    is_sequence_block_cell: bool,
    sequence_block_color: Color,
    is_sequence_area_cell: bool,
    sequence_area_bg: Color,
) -> Style {
    let mut style = style_for_highlight_flag(flag, has_active_selection_in_diagram, focus_owner);

    if flag & 0b111 == 0 {
        if is_sequence_area_cell {
            style = style.bg(sequence_area_bg);
        }
        if is_sequence_block_cell {
            style = style.fg(sequence_block_color);
        }
        if is_note_cell {
            style = style.fg(Color::DarkGray);
        }
    }

    style
}

fn objects_from_diagram(diagram: &Diagram) -> Vec<SelectableObject> {
    let diagram_id = diagram.diagram_id().clone();

    let mut objects = match diagram.ast() {
        DiagramAst::Sequence(ast) => objects_from_sequence_ast(&diagram_id, ast),
        DiagramAst::Flowchart(ast) => objects_from_flowchart_ast(&diagram_id, ast),
    };

    objects.sort_by_cached_key(|obj| obj.object_ref.to_string());
    objects
}

fn objects_from_sequence_ast(diagram_id: &DiagramId, ast: &SequenceAst) -> Vec<SelectableObject> {
    let participant_category = category_path(&["seq", "participant"]);
    let message_category = category_path(&["seq", "message"]);

    let mut out = Vec::new();

    for (participant_id, participant) in ast.participants() {
        let object_ref = ObjectRef::new(
            diagram_id.clone(),
            participant_category.clone(),
            participant_id.clone(),
        );
        out.push(SelectableObject {
            label: format!("participant {} ({})", participant_id, participant.mermaid_name()),
            note: participant.note().map(|note| note.to_owned()),
            object_ref,
        });
    }

    for msg in ast.messages() {
        let object_ref =
            ObjectRef::new(diagram_id.clone(), message_category.clone(), msg.message_id().clone());
        out.push(SelectableObject {
            label: format!(
                "message {} {}→{}: {}",
                msg.message_id(),
                msg.from_participant_id(),
                msg.to_participant_id(),
                msg.text()
            ),
            note: None,
            object_ref,
        });
    }

    out
}

fn objects_from_flowchart_ast(diagram_id: &DiagramId, ast: &FlowchartAst) -> Vec<SelectableObject> {
    let node_category = category_path(&["flow", "node"]);
    let edge_category = category_path(&["flow", "edge"]);

    let mut out = Vec::new();

    for (node_id, node) in ast.nodes() {
        let object_ref = ObjectRef::new(diagram_id.clone(), node_category.clone(), node_id.clone());
        out.push(SelectableObject {
            label: format!("node {} ({})", node_id, node.label()),
            note: node.note().map(|note| note.to_owned()),
            object_ref,
        });
    }

    for (edge_id, edge) in ast.edges() {
        let object_ref = ObjectRef::new(diagram_id.clone(), edge_category.clone(), edge_id.clone());
        out.push(SelectableObject {
            label: format!("edge {} {}→{}", edge_id, edge.from_node_id(), edge.to_node_id()),
            note: None,
            object_ref,
        });
    }

    out
}

fn category_path(segments: &[&str]) -> CategoryPath {
    CategoryPath::new(segments.iter().map(|s| (*s).to_owned()).collect())
        .expect("valid CategoryPath")
}

#[derive(Debug, Clone, Copy)]
struct SubsequenceStats {
    first: usize,
    span: usize,
    consecutive: usize,
    start_boundary: bool,
}

fn search_candidates_from_session(session: &Session) -> Vec<SearchCandidate> {
    let mut candidates = Vec::new();
    for diagram in session.diagrams().values() {
        for obj in objects_from_diagram(diagram) {
            let object_ref_text = obj.object_ref.to_string();
            let haystack = format!("{object_ref_text} {}", obj.label).to_lowercase();
            candidates.push(SearchCandidate { haystack, object_ref: obj.object_ref });
        }
    }

    candidates.sort_by(|a, b| a.haystack.cmp(&b.haystack));
    candidates
}

fn ranked_search_results(
    candidates: &[SearchCandidate],
    query: &str,
    kind: SearchKind,
    active_diagram_id: Option<&DiagramId>,
) -> Vec<ObjectRef> {
    let needle = query.trim();
    if needle.is_empty() {
        return Vec::new();
    }

    let needle = needle.to_lowercase();
    let mut groups = BTreeMap::<String, Vec<(i64, usize)>>::new();
    for (idx, candidate) in candidates.iter().enumerate() {
        let score = match kind {
            SearchKind::Regular => regular_score(&needle, &candidate.haystack),
            SearchKind::Fuzzy => fuzzy_score(&needle, &candidate.haystack),
        };
        let Some(score) = score else {
            continue;
        };
        groups.entry(candidate.object_ref.diagram_id().to_string()).or_default().push((score, idx));
    }

    if groups.is_empty() {
        return Vec::new();
    }

    for matches in groups.values_mut() {
        matches.sort_by(|(score_a, idx_a), (score_b, idx_b)| {
            score_b
                .cmp(score_a)
                .then_with(|| candidates[*idx_a].haystack.cmp(&candidates[*idx_b].haystack))
        });
    }

    let active_diagram = active_diagram_id.map(ToString::to_string);
    let mut ordered_groups = groups
        .into_iter()
        .map(|(diagram_id, matches)| {
            let best_score = matches.first().map(|(score, _)| *score).unwrap_or(i64::MIN);
            (diagram_id, best_score, matches)
        })
        .collect::<Vec<_>>();

    ordered_groups.sort_by(|(diagram_a, best_a, _), (diagram_b, best_b, _)| {
        let a_is_active = active_diagram.as_deref() == Some(diagram_a.as_str());
        let b_is_active = active_diagram.as_deref() == Some(diagram_b.as_str());
        b_is_active
            .cmp(&a_is_active)
            .then_with(|| best_b.cmp(best_a))
            .then_with(|| diagram_a.cmp(diagram_b))
    });

    let mut out = Vec::new();
    for (_, _, matches) in ordered_groups {
        for (_, idx) in matches {
            out.push(candidates[idx].object_ref.clone());
        }
    }
    out
}

fn regular_score(needle: &str, haystack: &str) -> Option<i64> {
    let needle = needle.trim();
    if needle.is_empty() {
        return None;
    }

    let first = haystack.find(needle)?;
    let starts = first == 0;
    let start_boundary =
        if starts { true } else { haystack[..first].chars().last().is_some_and(is_boundary_char) };
    let occurrences = haystack.match_indices(needle).count() as i64;

    let mut score = 200_000i64.saturating_sub((first as i64) * 1000);
    score += occurrences * 200;
    score -= haystack.chars().count() as i64;
    if starts {
        score += 50_000;
    }
    if start_boundary {
        score += 20_000;
    }
    if haystack == needle {
        score += 100_000;
    }

    Some(score)
}

fn fuzzy_score(needle: &str, haystack: &str) -> Option<i64> {
    let needle = needle.trim();
    if needle.is_empty() {
        return None;
    }

    let subseq = subsequence_stats(needle, haystack)?;
    let ratio = rapidfuzz::fuzz::ratio(needle.chars(), haystack.chars());
    let ratio_score = (ratio * 1000.0).round() as i64;

    let mut score = ratio_score;
    score -= subseq.span as i64;
    score -= (subseq.first as i64) / 4;
    score += (subseq.consecutive as i64) * 40;
    if subseq.start_boundary {
        score += 150;
    }
    if haystack.contains(needle) {
        score += 2000;
    } else {
        score += 500;
    }

    Some(score)
}

fn subsequence_stats(needle: &str, haystack: &str) -> Option<SubsequenceStats> {
    let mut needle_iter = needle.chars().peekable();
    let mut first: Option<usize> = None;
    let mut last: usize = 0;
    let mut prev_match: Option<usize> = None;
    let mut consecutive: usize = 0;
    let mut start_boundary = false;
    let mut prev_hay: Option<char> = None;

    for (idx, ch) in haystack.chars().enumerate() {
        let Some(&want) = needle_iter.peek() else {
            break;
        };

        if ch == want {
            needle_iter.next();

            if first.is_none() {
                first = Some(idx);
                start_boundary = prev_hay.map_or(true, is_boundary_char);
            }

            if let Some(prev) = prev_match {
                if idx == prev + 1 {
                    consecutive += 1;
                }
            }
            prev_match = Some(idx);
            last = idx;
        }

        prev_hay = Some(ch);
    }

    if needle_iter.peek().is_some() {
        return None;
    }

    let first = first?;
    Some(SubsequenceStats {
        first,
        span: last.saturating_sub(first).saturating_add(1),
        consecutive,
        start_boundary,
    })
}

fn is_boundary_char(ch: char) -> bool {
    matches!(ch, '/' | ':' | '-' | '_' | ' ')
}

fn xrefs_from_session(session: &Session) -> Vec<SelectableXRef> {
    session
        .xrefs()
        .iter()
        .map(|(xref_id, xref)| {
            let label = match xref.label() {
                Some(text) => format!(
                    "{} {} {}: {} ({} → {})",
                    xref_id,
                    xref.status(),
                    xref.kind(),
                    text,
                    xref.from(),
                    xref.to()
                ),
                None => format!(
                    "{} {} {}: {} → {}",
                    xref_id,
                    xref.status(),
                    xref.kind(),
                    xref.from(),
                    xref.to()
                ),
            };

            SelectableXRef { xref_id: xref_id.clone(), label, xref: xref.clone() }
        })
        .collect()
}

fn xref_direction_prefix(selected: Option<&ObjectRef>, xref: &XRef) -> &'static str {
    let Some(selected) = selected else {
        return "";
    };

    let has_outgoing = xref.from() == selected;
    let has_incoming = xref.to() == selected;

    xref_direction_prefix_for_flags(has_outgoing, has_incoming)
}

fn xref_direction_prefix_for_flags(has_outgoing: bool, has_incoming: bool) -> &'static str {
    match (has_outgoing, has_incoming) {
        (true, true) => "▾▴ ",
        (true, false) => "▴ ",
        (false, true) => "▾ ",
        (false, false) => "",
    }
}

pub fn demo_session() -> Session {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data").join("demo-session");
    let folder = SessionFolder::new(root.clone());

    match folder.load_session() {
        Ok(session) => session,
        Err(err) => {
            if cfg!(test) {
                panic!("failed to load demo session fixture from {}: {err}", root.display());
            }
            eprintln!(
                "warning: failed to load demo session fixture from {}: {err}; falling back to built-in demo session",
                root.display()
            );
            demo_session_fallback()
        }
    }
}

fn demo_session_fallback() -> Session {
    fn oid(value: &str) -> ObjectId {
        ObjectId::new(value).expect("object id")
    }

    let mut session = Session::new(SessionId::new("s:demo").expect("session id"));

    let mut seq_ast = SequenceAst::default();
    let p_alice = oid("p:alice");
    let p_bob = oid("p:bob");
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:0001"),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));

    let seq_id = DiagramId::new("demo-seq").expect("diagram id");
    let seq_diagram = Diagram::new(seq_id.clone(), "Sequence demo", DiagramAst::Sequence(seq_ast));

    let flow_ast = crate::model::fixtures::flowchart_small_dag();
    let n_a = oid("n:a");
    let n_b = oid("n:b");

    let flow_id = DiagramId::new("demo-flow").expect("diagram id");
    let flow_diagram =
        Diagram::new(flow_id.clone(), "Flowchart demo", DiagramAst::Flowchart(flow_ast));

    session.diagrams_mut().insert(seq_id, seq_diagram);
    session.diagrams_mut().insert(flow_id.clone(), flow_diagram);

    let x1 = XRef::new(
        ObjectRef::new(flow_id.clone(), category_path(&["flow", "node"]), n_a.clone()),
        ObjectRef::new(flow_id.clone(), category_path(&["flow", "edge"]), oid("e:ab")),
        "uses",
        XRefStatus::Ok,
    );

    let x2 = XRef::new(
        ObjectRef::new(flow_id.clone(), category_path(&["flow", "node"]), n_b.clone()),
        ObjectRef::new(flow_id, category_path(&["flow", "edge"]), oid("e:missing")),
        "uses",
        XRefStatus::DanglingTo,
    );

    session.xrefs_mut().insert(XRefId::new("x:1").expect("xref id"), x1);
    session.xrefs_mut().insert(XRefId::new("x:2").expect("xref id"), x2);

    session
}

#[cfg(test)]
pub(crate) mod testing {
    use super::{App, Session, SessionFolder, UiState};
    use crate::model::ObjectRef;
    use crossterm::event::KeyCode;
    use ratatui::prelude::Text;
    use std::{collections::BTreeSet, sync::Arc};
    use tokio::sync::Mutex;

    pub(crate) struct HeadlessTui {
        app: App,
    }

    impl HeadlessTui {
        pub(crate) fn new(
            session: Session,
            agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
            ui_state: Option<Arc<Mutex<UiState>>>,
            session_folder: Option<SessionFolder>,
        ) -> Self {
            let mut app = App::new_with_ui(session, agent_highlights);
            app.ui_state = ui_state;
            app.session_folder = session_folder;
            app.publish_focus_to_ui_state();
            Self { app }
        }

        pub(crate) fn press(&mut self, code: KeyCode) -> bool {
            self.app.handle_key_code(code)
        }

        #[allow(dead_code)]
        pub(crate) fn sync_from_ui_state(&mut self) {
            self.app.sync_from_ui_state();
        }

        pub(crate) fn selected_ref(&self) -> Option<ObjectRef> {
            self.app.selected_ref().cloned()
        }

        pub(crate) fn diagram_text(&self) -> Text<'static> {
            self.app.diagram_text()
        }
    }
}

#[cfg(test)]
mod tests;
