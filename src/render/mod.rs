// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Rendering for diagrams and walkthroughs.
//!
//! Renderers produce Unicode/ASCII text output as well as a stable highlight index that the TUI
//! and MCP integrations can use for cell-accurate selection/highlighting.

use std::collections::BTreeMap;
use std::fmt;

use crate::model::ObjectRef;

pub mod diagram;
pub mod flowchart;
pub mod sequence;
#[cfg(test)]
mod test_utils;
mod text;
pub mod walkthrough;

pub use diagram::{render_diagram_unicode, render_diagram_unicode_annotated, DiagramRenderError};
pub use flowchart::{
    render_flowchart_unicode, render_flowchart_unicode_annotated, FlowchartRenderError,
};
pub use sequence::{
    render_sequence_unicode, render_sequence_unicode_annotated, SequenceRenderError,
};
pub use walkthrough::{render_walkthrough_unicode, WalkthroughRenderError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderOptions {
    pub show_notes: bool,
    pub prefix_object_labels: bool,
    pub flowchart_extra_col_gap: usize,
}

/// A contiguous span of highlighted cells within a single rendered line.
///
/// Coordinates are `(y, x0, x1)` in character-cell indices, inclusive, relative to the returned
/// rendered text lines.
pub type LineSpan = (usize, usize, usize);

/// Mapping from stable object references to the spans that should be highlighted for that object.
pub type HighlightIndex = BTreeMap<ObjectRef, Vec<LineSpan>>;

/// Render output plus an index suitable for stable, cell-accurate UI highlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedRender {
    pub text: String,
    pub highlight_index: HighlightIndex,
}

pub(crate) fn clamp_highlight_index_to_text(highlight_index: &mut HighlightIndex, text: &str) {
    let lines = text.split('\n').collect::<Vec<_>>();
    let mut line_lens = Vec::<usize>::with_capacity(lines.len());
    for line in &lines {
        line_lens.push(text::text_len(line));
    }

    highlight_index.retain(|_, spans| {
        spans.retain_mut(|span| {
            let (y, x0, x1) = span;

            let len = match line_lens.get(*y) {
                Some(len) => *len,
                None => return false,
            };

            if len == 0 {
                return false;
            }

            if *x0 >= len {
                return false;
            }

            let max_x = len - 1;
            if *x1 > max_x {
                *x1 = max_x;
            }

            *x0 <= *x1
        });
        !spans.is_empty()
    });
}

pub const UNICODE_BOX_HORIZONTAL: char = '─';
pub const UNICODE_BOX_VERTICAL: char = '│';
pub const UNICODE_BOX_TOP_LEFT: char = '┌';
pub const UNICODE_BOX_TOP_RIGHT: char = '┐';
pub const UNICODE_BOX_BOTTOM_LEFT: char = '└';
pub const UNICODE_BOX_BOTTOM_RIGHT: char = '┘';
pub const UNICODE_BOX_TEE_RIGHT: char = '├';
pub const UNICODE_BOX_TEE_LEFT: char = '┤';
pub const UNICODE_BOX_TEE_DOWN: char = '┬';
pub const UNICODE_BOX_TEE_UP: char = '┴';
pub const UNICODE_BOX_CROSS: char = '┼';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoxEdges(u8);

impl BoxEdges {
    const NONE: Self = Self(0);
    const LEFT: Self = Self(1 << 0);
    const RIGHT: Self = Self(1 << 1);
    const UP: Self = Self(1 << 2);
    const DOWN: Self = Self(1 << 3);

    fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn contains(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

fn box_edges_from_char(ch: char) -> Option<BoxEdges> {
    match ch {
        UNICODE_BOX_HORIZONTAL => Some(BoxEdges::LEFT.union(BoxEdges::RIGHT)),
        UNICODE_BOX_VERTICAL => Some(BoxEdges::UP.union(BoxEdges::DOWN)),
        UNICODE_BOX_TOP_LEFT => Some(BoxEdges::RIGHT.union(BoxEdges::DOWN)),
        UNICODE_BOX_TOP_RIGHT => Some(BoxEdges::LEFT.union(BoxEdges::DOWN)),
        UNICODE_BOX_BOTTOM_LEFT => Some(BoxEdges::RIGHT.union(BoxEdges::UP)),
        UNICODE_BOX_BOTTOM_RIGHT => Some(BoxEdges::LEFT.union(BoxEdges::UP)),
        UNICODE_BOX_TEE_RIGHT => Some(BoxEdges::UP.union(BoxEdges::DOWN).union(BoxEdges::RIGHT)),
        UNICODE_BOX_TEE_LEFT => Some(BoxEdges::UP.union(BoxEdges::DOWN).union(BoxEdges::LEFT)),
        UNICODE_BOX_TEE_DOWN => Some(BoxEdges::LEFT.union(BoxEdges::RIGHT).union(BoxEdges::DOWN)),
        UNICODE_BOX_TEE_UP => Some(BoxEdges::LEFT.union(BoxEdges::RIGHT).union(BoxEdges::UP)),
        UNICODE_BOX_CROSS => Some(
            BoxEdges::LEFT
                .union(BoxEdges::RIGHT)
                .union(BoxEdges::UP)
                .union(BoxEdges::DOWN),
        ),
        _ => None,
    }
}

fn box_char_from_edges(edges: BoxEdges) -> char {
    match edges.0 {
        // Empty shouldn't normally occur for box cells; treat as blank.
        0 => ' ',
        // Straight segments (including endpoints).
        1..=3 => UNICODE_BOX_HORIZONTAL,
        4 | 8 | 12 => UNICODE_BOX_VERTICAL,
        // Corners.
        10 => UNICODE_BOX_TOP_LEFT,
        9 => UNICODE_BOX_TOP_RIGHT,
        6 => UNICODE_BOX_BOTTOM_LEFT,
        5 => UNICODE_BOX_BOTTOM_RIGHT,
        // Tees.
        14 => UNICODE_BOX_TEE_RIGHT,
        13 => UNICODE_BOX_TEE_LEFT,
        11 => UNICODE_BOX_TEE_DOWN,
        7 => UNICODE_BOX_TEE_UP,
        // Cross.
        15 => UNICODE_BOX_CROSS,
        // Unreachable with 4 bits; keep a deterministic fallback.
        _ => UNICODE_BOX_CROSS,
    }
}

/// A fixed-size, bounds-checked character grid.
///
/// Collision behavior is deterministic:
/// - non-box characters overwrite (last writer wins)
/// - Unicode box-drawing characters merge into junctions (`┼`, `├`, `┤`, `┬`, `┴`) instead of overwriting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<char>,
    box_edges: Vec<BoxEdges>,
}

impl Canvas {
    /// Creates a new canvas filled with spaces (`' '`).
    pub fn new(width: usize, height: usize) -> Result<Self, CanvasError> {
        Self::new_filled(width, height, ' ')
    }

    /// Creates a new canvas filled with `fill`.
    pub fn new_filled(width: usize, height: usize, fill: char) -> Result<Self, CanvasError> {
        let len = width
            .checked_mul(height)
            .ok_or(CanvasError::AreaOverflow { width, height })?;

        Ok(Self {
            width,
            height,
            cells: vec![fill; len],
            box_edges: vec![BoxEdges::NONE; len],
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    /// Returns the character at `(x, y)`.
    pub fn get(&self, x: usize, y: usize) -> Result<char, CanvasError> {
        let idx = self.index_of(x, y)?;
        Ok(self.render_at(x, y, idx))
    }

    pub(crate) fn has_box_vertical(&self, x: usize, y: usize) -> Result<bool, CanvasError> {
        let idx = self.index_of(x, y)?;
        let edges = self.box_edges[idx];
        Ok(edges.contains(BoxEdges::UP) || edges.contains(BoxEdges::DOWN))
    }

    /// Sets the character at `(x, y)`.
    pub fn set(&mut self, x: usize, y: usize, ch: char) -> Result<(), CanvasError> {
        let idx = self.index_of(x, y)?;
        if let Some(edges) = box_edges_from_char(ch) {
            self.box_edges[idx] = self.box_edges[idx].union(edges);
        } else {
            self.cells[idx] = ch;
            self.box_edges[idx] = BoxEdges::NONE;
        }
        Ok(())
    }

    /// Fills the entire canvas with `ch`.
    pub fn fill(&mut self, ch: char) {
        self.cells.fill(ch);
        self.box_edges.fill(BoxEdges::NONE);
    }

    /// Writes `text` left-to-right starting at `(x, y)`.
    ///
    /// Behavior:
    /// - If `y` is out of bounds: returns an error.
    /// - If `text` exceeds the row: clips at the right edge.
    pub fn write_str(&mut self, x: usize, y: usize, text: &str) -> Result<(), CanvasError> {
        if y >= self.height {
            return Err(CanvasError::OutOfBounds {
                x,
                y,
                width: self.width,
                height: self.height,
            });
        }

        let mut x = x;
        for ch in text.chars() {
            if x >= self.width {
                break;
            }
            self.set(x, y, ch)?;
            x += 1;
        }

        Ok(())
    }

    /// Draws a Unicode box-drawing horizontal line from `x0..=x1` at `y`.
    pub fn draw_hline(&mut self, x0: usize, x1: usize, y: usize) -> Result<(), CanvasError> {
        let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };

        if y >= self.height {
            return Err(CanvasError::OutOfBounds {
                x: min_x,
                y,
                width: self.width,
                height: self.height,
            });
        }

        if max_x >= self.width {
            return Err(CanvasError::OutOfBounds {
                x: max_x,
                y,
                width: self.width,
                height: self.height,
            });
        }

        for x in min_x..=max_x {
            self.set(x, y, UNICODE_BOX_HORIZONTAL)?;
        }

        Ok(())
    }

    /// Draws a Unicode box-drawing vertical line from `y0..=y1` at `x`.
    pub fn draw_vline(&mut self, x: usize, y0: usize, y1: usize) -> Result<(), CanvasError> {
        let (min_y, max_y) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };

        if x >= self.width {
            return Err(CanvasError::OutOfBounds {
                x,
                y: min_y,
                width: self.width,
                height: self.height,
            });
        }

        if max_y >= self.height {
            return Err(CanvasError::OutOfBounds {
                x,
                y: max_y,
                width: self.width,
                height: self.height,
            });
        }

        for y in min_y..=max_y {
            self.set(x, y, UNICODE_BOX_VERTICAL)?;
        }

        Ok(())
    }

    /// Draws a Unicode single-line box with corners at `(x0, y0)` and `(x1, y1)`.
    pub fn draw_box(
        &mut self,
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
    ) -> Result<(), CanvasError> {
        let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (min_y, max_y) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };

        if max_x >= self.width {
            return Err(CanvasError::OutOfBounds {
                x: max_x,
                y: min_y,
                width: self.width,
                height: self.height,
            });
        }

        if max_y >= self.height {
            return Err(CanvasError::OutOfBounds {
                x: min_x,
                y: max_y,
                width: self.width,
                height: self.height,
            });
        }

        if min_x == max_x && min_y == max_y {
            return self.set(min_x, min_y, UNICODE_BOX_CROSS);
        }

        if min_y == max_y {
            return self.draw_hline(min_x, max_x, min_y);
        }

        if min_x == max_x {
            return self.draw_vline(min_x, min_y, max_y);
        }

        for x in (min_x + 1)..max_x {
            self.set(x, min_y, UNICODE_BOX_HORIZONTAL)?;
            self.set(x, max_y, UNICODE_BOX_HORIZONTAL)?;
        }

        for y in (min_y + 1)..max_y {
            self.set(min_x, y, UNICODE_BOX_VERTICAL)?;
            self.set(max_x, y, UNICODE_BOX_VERTICAL)?;
        }

        self.set(min_x, min_y, UNICODE_BOX_TOP_LEFT)?;
        self.set(max_x, min_y, UNICODE_BOX_TOP_RIGHT)?;
        self.set(min_x, max_y, UNICODE_BOX_BOTTOM_LEFT)?;
        self.set(max_x, max_y, UNICODE_BOX_BOTTOM_RIGHT)?;

        Ok(())
    }

    fn index_of(&self, x: usize, y: usize) -> Result<usize, CanvasError> {
        if !self.in_bounds(x, y) {
            return Err(CanvasError::OutOfBounds {
                x,
                y,
                width: self.width,
                height: self.height,
            });
        }

        Ok((y * self.width) + x)
    }

    fn render_at(&self, x: usize, y: usize, idx: usize) -> char {
        let edges = self.box_edges[idx];
        if edges.is_empty() {
            return self.cells[idx];
        }

        let connected = self.connected_box_edges(x, y, edges);
        let edges_for_render = if connected.is_empty() {
            edges
        } else {
            connected
        };
        box_char_from_edges(edges_for_render)
    }

    fn connected_box_edges(&self, x: usize, y: usize, edges: BoxEdges) -> BoxEdges {
        let mut connected = BoxEdges::NONE;

        if edges.contains(BoxEdges::LEFT) && x > 0 {
            let left_idx = (y * self.width) + (x - 1);
            if self.box_edges[left_idx].contains(BoxEdges::RIGHT) {
                connected = connected.union(BoxEdges::LEFT);
            }
        }

        if edges.contains(BoxEdges::RIGHT) && (x + 1) < self.width {
            let right_idx = (y * self.width) + (x + 1);
            if self.box_edges[right_idx].contains(BoxEdges::LEFT) {
                connected = connected.union(BoxEdges::RIGHT);
            }
        }

        if edges.contains(BoxEdges::UP) && y > 0 {
            let up_idx = ((y - 1) * self.width) + x;
            if self.box_edges[up_idx].contains(BoxEdges::DOWN) {
                connected = connected.union(BoxEdges::UP);
            }
        }

        if edges.contains(BoxEdges::DOWN) && (y + 1) < self.height {
            let down_idx = ((y + 1) * self.width) + x;
            if self.box_edges[down_idx].contains(BoxEdges::UP) {
                connected = connected.union(BoxEdges::DOWN);
            }
        }

        connected
    }
}

impl fmt::Display for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Write as _;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width) + x;
                let ch = self.render_at(x, y, idx);
                f.write_char(ch)?;
            }

            if y + 1 < self.height {
                f.write_char('\n')?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasError {
    AreaOverflow {
        width: usize,
        height: usize,
    },
    OutOfBounds {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    },
}

impl fmt::Display for CanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AreaOverflow { width, height } => {
                write!(f, "canvas area overflow: {width}*{height}")
            }
            Self::OutOfBounds {
                x,
                y,
                width,
                height,
            } => {
                write!(f, "out of bounds: ({x},{y}) for {width}x{height} canvas")
            }
        }
    }
}

impl std::error::Error for CanvasError {}

#[cfg(test)]
mod tests {
    use super::{Canvas, CanvasError};

    #[test]
    fn set_and_get_in_bounds() {
        let mut c = Canvas::new_filled(3, 2, '.').expect("canvas");
        assert_eq!(c.get(1, 0).unwrap(), '.');
        c.set(1, 0, 'X').unwrap();
        assert_eq!(c.get(1, 0).unwrap(), 'X');
        assert_eq!(c.to_string(), ".X.\n...");
    }

    #[test]
    fn set_out_of_bounds_errors() {
        let mut c = Canvas::new(2, 2).expect("canvas");
        let err = c.set(2, 0, 'X').unwrap_err();
        assert_eq!(
            err,
            CanvasError::OutOfBounds {
                x: 2,
                y: 0,
                width: 2,
                height: 2
            }
        );
    }

    #[test]
    fn get_out_of_bounds_errors() {
        let c = Canvas::new(2, 2).expect("canvas");
        let err = c.get(0, 2).unwrap_err();
        assert_eq!(
            err,
            CanvasError::OutOfBounds {
                x: 0,
                y: 2,
                width: 2,
                height: 2
            }
        );
    }

    #[test]
    fn write_str_clips_at_right_edge() {
        let mut c = Canvas::new_filled(4, 1, '.').expect("canvas");
        c.write_str(2, 0, "abcdef").unwrap();
        assert_eq!(c.to_string(), "..ab");
    }

    #[test]
    fn rejects_area_overflow() {
        let err = Canvas::new_filled(usize::MAX, 2, '.').unwrap_err();
        assert_eq!(
            err,
            CanvasError::AreaOverflow {
                width: usize::MAX,
                height: 2
            }
        );
    }

    #[test]
    fn draw_hline_draws_unicode_horizontal() {
        let mut c = Canvas::new_filled(5, 3, '.').expect("canvas");
        c.draw_hline(1, 3, 1).unwrap();
        assert_eq!(c.to_string(), ".....\n.───.\n.....");
    }

    #[test]
    fn draw_vline_draws_unicode_vertical() {
        let mut c = Canvas::new_filled(5, 3, '.').expect("canvas");
        c.draw_vline(2, 0, 2).unwrap();
        assert_eq!(c.to_string(), "..│..\n..│..\n..│..");
    }

    #[test]
    fn draw_box_draws_unicode_corners_and_edges() {
        let mut c = Canvas::new_filled(6, 5, '.').expect("canvas");
        c.draw_box(1, 1, 4, 3).unwrap();
        assert_eq!(c.to_string(), "......\n.┌──┐.\n.│..│.\n.└──┘.\n......");
    }

    #[test]
    fn draw_box_out_of_bounds_is_not_partial() {
        let mut c = Canvas::new_filled(4, 3, '.').expect("canvas");
        let err = c.draw_box(0, 0, 4, 2).unwrap_err();
        assert_eq!(
            err,
            CanvasError::OutOfBounds {
                x: 4,
                y: 0,
                width: 4,
                height: 3
            }
        );
        assert_eq!(c.to_string(), "....\n....\n....");
    }

    #[test]
    fn intersects_hline_and_vline_as_cross_not_overwrite() {
        let mut c = Canvas::new_filled(5, 5, '.').expect("canvas");
        c.draw_hline(0, 4, 2).unwrap();
        c.draw_vline(2, 0, 4).unwrap();
        assert_eq!(c.to_string(), "..│..\n..│..\n──┼──\n..│..\n..│..");
    }

    #[test]
    fn intersects_as_left_and_right_tees() {
        let mut c = Canvas::new_filled(5, 5, '.').expect("canvas");
        c.draw_vline(2, 0, 4).unwrap();
        c.draw_hline(2, 4, 2).unwrap();
        assert_eq!(c.to_string(), "..│..\n..│..\n..├──\n..│..\n..│..");

        let mut c = Canvas::new_filled(5, 5, '.').expect("canvas");
        c.draw_vline(2, 0, 4).unwrap();
        c.draw_hline(0, 2, 2).unwrap();
        assert_eq!(c.to_string(), "..│..\n..│..\n──┤..\n..│..\n..│..");
    }

    #[test]
    fn intersects_as_top_and_bottom_tees() {
        let mut c = Canvas::new_filled(5, 5, '.').expect("canvas");
        c.draw_hline(0, 4, 2).unwrap();
        c.draw_vline(2, 2, 4).unwrap();
        assert_eq!(c.to_string(), ".....\n.....\n──┬──\n..│..\n..│..");

        let mut c = Canvas::new_filled(5, 5, '.').expect("canvas");
        c.draw_hline(0, 4, 2).unwrap();
        c.draw_vline(2, 0, 2).unwrap();
        assert_eq!(c.to_string(), "..│..\n..│..\n──┴──\n.....\n.....");
    }
}
