use std::path::PathBuf;

use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::model::Kind;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

/// An entry in the start screen's Recent list.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Recent {
    Diff(PathBuf),
    Compare(PathBuf, PathBuf),
}

impl Recent {
    pub fn title(&self) -> String {
        let name = |p: &PathBuf| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match self {
            Recent::Diff(p) => name(p),
            Recent::Compare(a, b) => format!("{}  ↔  {}", name(a), name(b)),
        }
    }
    pub fn subtitle(&self) -> String {
        let dir = |p: &PathBuf| {
            let d = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
            match std::env::var("HOME") {
                Ok(h) if d.starts_with(&h) => format!("~{}", &d[h.len()..]),
                _ => d,
            }
        };
        match self {
            Recent::Diff(p) => dir(p),
            Recent::Compare(a, _) => dir(a),
        }
    }
    pub fn exists(&self) -> bool {
        match self {
            Recent::Diff(p) => p.is_file(),
            Recent::Compare(a, b) => a.is_file() && b.is_file(),
        }
    }
}

impl ThemeChoice {
    pub fn apply(self, ctx: &egui::Context) {
        match self {
            ThemeChoice::System => ctx.set_theme(egui::ThemePreference::System),
            ThemeChoice::Light => ctx.set_theme(egui::Theme::Light),
            ThemeChoice::Dark => ctx.set_theme(egui::Theme::Dark),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // Kompare's default palette (removed = green, changed = red, added = blue, applied = yellow)
    pub remove_color: Color32,
    pub change_color: Color32,
    pub add_color: Color32,
    pub applied_color: Color32,
    pub font_size: f32,
    pub wrap: bool,
    pub tab_width: usize,
    pub context_lines: usize,
    pub show_text_view: bool,
    pub show_nav: bool,
    pub theme: ThemeChoice,
    pub recents: Vec<Recent>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            remove_color: Color32::from_rgb(190, 237, 190),
            change_color: Color32::from_rgb(237, 190, 190),
            add_color: Color32::from_rgb(190, 190, 237),
            applied_color: Color32::from_rgb(237, 237, 190),
            font_size: 12.0,
            wrap: true,
            tab_width: 4,
            context_lines: 3,
            show_text_view: false,
            show_nav: true,
            theme: ThemeChoice::System,
            recents: Vec::new(),
        }
    }
}

impl Settings {
    pub fn push_recent(&mut self, r: Recent) {
        self.recents.retain(|x| *x != r);
        self.recents.insert(0, r);
        self.recents.truncate(12);
    }

    /// Base block color for a difference kind, adapted to the current theme.
    pub fn color_for(&self, kind: Kind, dark: bool) -> Option<Color32> {
        let c = match kind {
            Kind::Unchanged => return None,
            Kind::Change => self.change_color,
            Kind::Insert => self.add_color,
            Kind::Delete => self.remove_color,
        };
        Some(if dark { to_dark(c) } else { c })
    }

    /// Slightly lighter variant used for the selected difference (Kompare: `color.lighter(110)`).
    pub fn selected_color(c: Color32, dark: bool) -> Color32 {
        scale(c, if dark { 1.18 } else { 1.08 })
    }

    /// Darker tint used for intra-line changed characters.
    pub fn mark_color(c: Color32, dark: bool) -> Color32 {
        scale(c, if dark { 1.45 } else { 0.82 })
    }

    /// Outline color for the selected block / band.
    pub fn outline_color(c: Color32, dark: bool) -> Color32 {
        scale(c, if dark { 2.0 } else { 0.6 })
    }
}

fn to_dark(c: Color32) -> Color32 {
    // Keep the hue, drop the brightness so pastel colors work on a dark background.
    scale(c, 0.42)
}

pub fn scale(c: Color32, f: f32) -> Color32 {
    let s = |v: u8| ((v as f32) * f).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(s(c.r()), s(c.g()), s(c.b()))
}
