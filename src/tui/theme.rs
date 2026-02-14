// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::{env, error::Error, fmt};

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub(crate) struct TuiTheme {
    palette: Option<TuiPalette>,
}

impl Default for TuiTheme {
    fn default() -> Self {
        Self { palette: None }
    }
}

impl TuiTheme {
    pub(crate) fn from_env() -> Result<Self, ThemeError> {
        let palette = palette_override_from_env()?;
        Ok(Self { palette })
    }

    pub(crate) fn base_style(&self) -> Style {
        match &self.palette {
            Some(palette) => Style::default().fg(palette.fg).bg(palette.bg),
            None => Style::default(),
        }
    }

    fn ansi_color(&self, color: Ansi16) -> Color {
        match &self.palette {
            Some(palette) => palette.ansi_color(color.idx()),
            None => color.into(),
        }
    }

    pub(crate) fn panel_border_style(&self, focused: bool) -> Style {
        if focused {
            self.base_style().fg(self.ansi_color(Ansi16::Yellow))
        } else {
            self.base_style()
        }
    }

    pub(crate) fn selection_style(&self) -> Style {
        self.base_style()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
    }

    pub(crate) fn error_style(&self) -> Style {
        self.base_style().fg(self.ansi_color(Ansi16::Red))
    }

    pub(crate) fn highlight_style(&self, flag: u8) -> Style {
        let base = self
            .base_style()
            .fg(self.ansi_color(Ansi16::Black))
            .add_modifier(Modifier::BOLD);
        match flag & 0b11 {
            0b01 => base.bg(self.ansi_color(Ansi16::Yellow)),
            0b10 => base.bg(self.ansi_color(Ansi16::Cyan)),
            0b11 => base.bg(self.ansi_color(Ansi16::Magenta)),
            _ => Style::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct TuiPalette {
    fg: Color,
    bg: Color,
    ansi: [Color; 16],
}

impl TuiPalette {
    const CSV_LEN: usize = 18;

    fn parse_csv(value: &str) -> Result<Self, String> {
        let parts: Vec<&str> = value.split(',').map(|part| part.trim()).collect();
        if parts.len() != Self::CSV_LEN {
            return Err(format!(
                "expected {} comma-separated colors (fg,bg,black,red,green,yellow,blue,magenta,cyan,white,bright_black,bright_red,bright_green,bright_yellow,bright_blue,bright_magenta,bright_cyan,bright_white), got {}",
                Self::CSV_LEN,
                parts.len()
            ));
        }

        let fg = parse_palette_color(parts[0])?;
        let bg = parse_palette_color(parts[1])?;

        let mut ansi = [Color::Reset; 16];
        for (idx, part) in parts.iter().skip(2).enumerate() {
            ansi[idx] = parse_palette_color(part)?;
        }

        Ok(Self { fg, bg, ansi })
    }

    fn ansi_color(&self, idx: usize) -> Color {
        self.ansi[idx]
    }
}

fn palette_override_from_env() -> Result<Option<TuiPalette>, ThemeError> {
    let (name, value) = match env::var("NEREID_TUI_PALETTE") {
        Ok(value) => ("NEREID_TUI_PALETTE", value),
        Err(env::VarError::NotPresent) => match env::var("NEREID_PALETTE") {
            Ok(value) => ("NEREID_PALETTE", value),
            Err(env::VarError::NotPresent) => return Ok(None),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(ThemeError::InvalidEnv {
                    name: "NEREID_PALETTE".to_string(),
                    value: "<non-unicode>".to_string(),
                });
            }
        },
        Err(env::VarError::NotUnicode(_)) => {
            return Err(ThemeError::InvalidEnv {
                name: "NEREID_TUI_PALETTE".to_string(),
                value: "<non-unicode>".to_string(),
            });
        }
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let parsed = TuiPalette::parse_csv(trimmed).map_err(|error| ThemeError::InvalidEnv {
        name: name.to_string(),
        value: format!("{trimmed} ({error})"),
    })?;

    Ok(Some(parsed))
}

fn parse_palette_color(value: &str) -> Result<Color, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("empty color".to_string());
    }

    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("rgb:") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() != 3 {
            return Err(format!("invalid rgb: value: {trimmed}"));
        }
        let r = parse_hex_channel(parts[0])?;
        let g = parse_hex_channel(parts[1])?;
        let b = parse_hex_channel(parts[2])?;
        return Ok(Color::Rgb(r, g, b));
    }

    let hex = trimmed
        .strip_prefix('#')
        .or_else(|| trimmed.strip_prefix("0x"))
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("invalid hex color: {trimmed} (expected #RRGGBB)"));
    }
    let rgb = u32::from_str_radix(hex, 16).map_err(|_| format!("invalid hex color: {trimmed}"))?;
    let r = ((rgb >> 16) & 0xFF) as u8;
    let g = ((rgb >> 8) & 0xFF) as u8;
    let b = (rgb & 0xFF) as u8;
    Ok(Color::Rgb(r, g, b))
}

fn parse_hex_channel(value: &str) -> Result<u8, String> {
    let value = value.trim();
    if value.len() == 2 {
        let parsed =
            u8::from_str_radix(value, 16).map_err(|_| format!("invalid rgb: component {value}"))?;
        return Ok(parsed);
    }
    if value.len() == 4 {
        let parsed = u16::from_str_radix(value, 16)
            .map_err(|_| format!("invalid rgb: component {value}"))?;
        return Ok((parsed >> 8) as u8);
    }
    Err(format!(
        "invalid rgb: component {value} (expected 2 or 4 hex digits)"
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Ansi16 {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Ansi16 {
    const fn idx(self) -> usize {
        match self {
            Self::Black => 0,
            Self::Red => 1,
            Self::Green => 2,
            Self::Yellow => 3,
            Self::Blue => 4,
            Self::Magenta => 5,
            Self::Cyan => 6,
            Self::White => 7,
            Self::BrightBlack => 8,
            Self::BrightRed => 9,
            Self::BrightGreen => 10,
            Self::BrightYellow => 11,
            Self::BrightBlue => 12,
            Self::BrightMagenta => 13,
            Self::BrightCyan => 14,
            Self::BrightWhite => 15,
        }
    }
}

impl From<Ansi16> for Color {
    fn from(value: Ansi16) -> Self {
        match value {
            Ansi16::Black => Color::Black,
            Ansi16::Red => Color::Red,
            Ansi16::Green => Color::Green,
            Ansi16::Yellow => Color::Yellow,
            Ansi16::Blue => Color::Blue,
            Ansi16::Magenta => Color::Magenta,
            Ansi16::Cyan => Color::Cyan,
            Ansi16::White => Color::Gray,
            Ansi16::BrightBlack => Color::DarkGray,
            Ansi16::BrightRed => Color::LightRed,
            Ansi16::BrightGreen => Color::LightGreen,
            Ansi16::BrightYellow => Color::LightYellow,
            Ansi16::BrightBlue => Color::LightBlue,
            Ansi16::BrightMagenta => Color::LightMagenta,
            Ansi16::BrightCyan => Color::LightCyan,
            Ansi16::BrightWhite => Color::White,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ThemeError {
    InvalidEnv { name: String, value: String },
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnv { name, value } => write!(f, "invalid env {name}={value}"),
        }
    }
}

impl Error for ThemeError {}

#[cfg(test)]
mod tests {
    use super::TuiPalette;

    #[test]
    fn palette_override_parses_valid_csv() {
        let palette = TuiPalette::parse_csv(
            "#111111,#222222,#000000,#ff0000,#00ff00,#ffff00,#0000ff,#ff00ff,#00ffff,#ffffff,#1a1a1a,#ff1111,#11ff11,#ffff11,#1111ff,#ff11ff,#11ffff,#fefefe",
        )
        .expect("palette");

        assert_eq!(palette.fg, ratatui::style::Color::Rgb(0x11, 0x11, 0x11));
        assert_eq!(palette.bg, ratatui::style::Color::Rgb(0x22, 0x22, 0x22));
        assert_eq!(palette.ansi_color(0), ratatui::style::Color::Rgb(0, 0, 0));
        assert_eq!(
            palette.ansi_color(1),
            ratatui::style::Color::Rgb(0xff, 0, 0)
        );
        assert_eq!(
            palette.ansi_color(2),
            ratatui::style::Color::Rgb(0, 0xff, 0)
        );
        assert_eq!(
            palette.ansi_color(15),
            ratatui::style::Color::Rgb(0xfe, 0xfe, 0xfe)
        );
    }

    #[test]
    fn palette_override_rejects_invalid_csv() {
        let err = TuiPalette::parse_csv("nope").unwrap_err();
        assert!(err.contains("expected"));
    }
}
