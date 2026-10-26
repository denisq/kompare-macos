//! The Kompare main view: two line panes with a connecting-bands widget in between
//! (KompareListView + KompareConnectWidget + KompareSplitter).

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::model::{Difference, FileModel, Kind, Segment};
use crate::settings::Settings;

pub struct ViewState {
    pub scroll_y: f32,
    pub scroll_x: f32,
    pub scroll_to_selected: bool,
    pub content_w: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState { scroll_y: 0.0, scroll_x: 0.0, scroll_to_selected: false, content_w: 0.0 }
    }
}

pub enum ViewAction {
    None,
    Select(usize),
}

/// Per-segment vertical layout in document coordinates, for both sides.
struct SegLayout {
    seg: usize,
    change: Option<usize>,
    kind: Kind,
    is_hunk: bool,
    yl: f32,
    hl: f32,
    yr: f32,
    hr: f32,
    left_line_h: Vec<f32>,
    right_line_h: Vec<f32>,
}

const CONNECT_W: f32 = 36.0;
const SCROLLBAR_W: f32 = 12.0;
const PAD: f32 = 6.0;

pub fn show(
    ui: &mut egui::Ui,
    file: &FileModel,
    selected: Option<usize>,
    settings: &Settings,
    state: &mut ViewState,
) -> ViewAction {
    let mut action = ViewAction::None;
    let rect = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    let dark = ui.visuals().dark_mode;
    let font = FontId::monospace(settings.font_size);
    let row_h = ui.fonts(|f| f.row_height(&font)).ceil();
    let char_w = ui.fonts(|f| f.glyph_width(&font, '0'));
    let hunk_h = (row_h * 0.9).ceil();
    let text_color = ui.visuals().text_color();
    let weak_color = ui.visuals().weak_text_color();

    // ---- geometry ----
    let max_line_no = file
        .segments
        .iter()
        .filter_map(|s| match s {
            Segment::Diff(d) => Some((d.src_line + d.src.len()).max(d.dst_line + d.dst.len())),
            _ => None,
        })
        .max()
        .unwrap_or(1);
    let digits = max_line_no.to_string().len().max(2) as f32;
    let gutter_w = (digits + 1.0) * char_w + PAD;
    let pane_w = ((rect.width() - CONNECT_W - SCROLLBAR_W) / 2.0).max(50.0);
    let left = Rect::from_min_size(rect.min, Vec2::new(pane_w, rect.height()));
    let connect = Rect::from_min_size(Pos2::new(left.max.x, rect.min.y), Vec2::new(CONNECT_W, rect.height()));
    let right = Rect::from_min_size(Pos2::new(connect.max.x, rect.min.y), Vec2::new(pane_w, rect.height()));
    let scrollbar = Rect::from_min_size(Pos2::new(right.max.x, rect.min.y), Vec2::new(SCROLLBAR_W, rect.height()));
    let text_w = pane_w - gutter_w - 2.0 * PAD;
    let wrap_width = if settings.wrap { text_w.max(20.0) } else { f32::INFINITY };

    let expand = |s: &str| -> String {
        if s.contains('\t') {
            s.replace('\t', &" ".repeat(settings.tab_width))
        } else {
            s.to_string()
        }
    };
    let line_height = |ui: &egui::Ui, s: &str| -> f32 {
        if settings.wrap {
            let g = ui.fonts(|f| f.layout(expand(s), font.clone(), text_color, wrap_width));
            g.size().y.max(row_h)
        } else {
            row_h
        }
    };

    // ---- layout ----
    let mut layout: Vec<SegLayout> = Vec::with_capacity(file.segments.len());
    let mut yl = 0.0;
    let mut yr = 0.0;
    let mut change_counter = 0usize;
    for (i, seg) in file.segments.iter().enumerate() {
        match seg {
            Segment::HunkHeader(_) => {
                layout.push(SegLayout {
                    seg: i,
                    change: None,
                    kind: Kind::Unchanged,
                    is_hunk: true,
                    yl,
                    hl: hunk_h,
                    yr,
                    hr: hunk_h,
                    left_line_h: Vec::new(),
                    right_line_h: Vec::new(),
                });
                yl += hunk_h;
                yr += hunk_h;
            }
            Segment::Diff(d) => {
                let lh: Vec<f32> = d.src.iter().map(|s| line_height(ui, s)).collect();
                let rh: Vec<f32> = d.dst.iter().map(|s| line_height(ui, s)).collect();
                let hl: f32 = lh.iter().sum();
                let hr: f32 = rh.iter().sum();
                let change = if d.kind != Kind::Unchanged {
                    change_counter += 1;
                    Some(change_counter - 1)
                } else {
                    None
                };
                layout.push(SegLayout {
                    seg: i,
                    change,
                    kind: d.kind,
                    is_hunk: false,
                    yl,
                    hl,
                    yr,
                    hr,
                    left_line_h: lh,
                    right_line_h: rh,
                });
                yl += hl;
                yr += hr;
            }
        }
    }
    let total_l = yl;
    let total_r = yr;
    let view_h = rect.height();

    // Kompare keeps the selected difference aligned across both panes.
    let sel_layout = selected.and_then(|k| layout.iter().find(|l| l.change == Some(k)));
    let shift = sel_layout.map(|l| l.yr - l.yl).unwrap_or(0.0);

    if state.scroll_to_selected {
        if let Some(l) = sel_layout {
            state.scroll_y = l.yl - view_h / 3.0;
        }
        state.scroll_to_selected = false;
    }

    // ---- input ----
    let hovered = response.hovered() || response.dragged();
    if hovered {
        let delta = ui.input(|i| i.smooth_scroll_delta);
        state.scroll_y -= delta.y;
        if !settings.wrap {
            state.scroll_x -= delta.x;
        }
    }
    let min_scroll = (-shift).min(0.0);
    let max_scroll = (total_l - view_h).max(total_r - shift - view_h).max(min_scroll);
    let dragging_scrollbar = response.dragged()
        && response
            .interact_pointer_pos()
            .map(|p| scrollbar.contains(Pos2::new(p.x, scrollbar.center().y)) && p.x >= scrollbar.min.x)
            .unwrap_or(false);
    let total_for_bar = total_l.max(total_r).max(view_h);
    if dragging_scrollbar {
        let d = response.drag_delta().y;
        state.scroll_y += d * total_for_bar / view_h;
    }
    state.scroll_y = state.scroll_y.clamp(min_scroll, max_scroll);
    let max_x = (state.content_w - text_w).max(0.0);
    state.scroll_x = if settings.wrap { 0.0 } else { state.scroll_x.clamp(0.0, max_x) };
    let scroll_l = state.scroll_y;
    let scroll_r = state.scroll_y + shift;

    // ---- click to select ----
    if response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            let (doc_y, is_left) = if left.contains(p) {
                (p.y - rect.min.y + scroll_l, true)
            } else if right.contains(p) {
                (p.y - rect.min.y + scroll_r, false)
            } else {
                (f32::NAN, true)
            };
            if !doc_y.is_nan() {
                for l in &layout {
                    let (y, h) = if is_left { (l.yl, l.hl) } else { (l.yr, l.hr) };
                    if doc_y >= y && doc_y < y + h {
                        if let Some(k) = l.change {
                            action = ViewAction::Select(k);
                        }
                        break;
                    }
                }
            }
        }
    }

    // ---- painting ----
    let painter = ui.painter_at(rect);
    let pane_bg = ui.visuals().extreme_bg_color;
    let gutter_bg = if dark { Color32::from_gray(36) } else { Color32::from_gray(244) };
    let hunk_bg = if dark { Color32::from_gray(52) } else { Color32::from_gray(226) };
    let sep_color = ui.visuals().widgets.noninteractive.bg_stroke.color;

    let mut content_w: f32 = 0.0;

    for (pane_rect, is_left, scroll) in [(left, true, scroll_l), (right, false, scroll_r)] {
        let p = painter.with_clip_rect(pane_rect);
        p.rect_filled(pane_rect, 0.0, pane_bg);
        let gutter = Rect::from_min_size(pane_rect.min, Vec2::new(gutter_w, pane_rect.height()));
        p.rect_filled(gutter, 0.0, gutter_bg);
        p.line_segment([gutter.right_top(), gutter.right_bottom()], Stroke::new(1.0_f32, sep_color));

        for l in &layout {
            let (y0, h) = if is_left { (l.yl, l.hl) } else { (l.yr, l.hr) };
            let top = pane_rect.min.y + y0 - scroll;
            if top > pane_rect.max.y || top + h < pane_rect.min.y {
                continue;
            }
            if l.is_hunk {
                let r = Rect::from_min_size(Pos2::new(pane_rect.min.x, top), Vec2::new(pane_rect.width(), h));
                p.rect_filled(r, 0.0, hunk_bg);
                p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0_f32, sep_color));
                if let Segment::HunkHeader(text) = &file.segments[l.seg] {
                    p.text(
                        Pos2::new(pane_rect.min.x + gutter_w + PAD, top + h / 2.0),
                        Align2::LEFT_CENTER,
                        text,
                        FontId::monospace(settings.font_size * 0.85),
                        weak_color,
                    );
                }
                continue;
            }
            let Segment::Diff(d) = &file.segments[l.seg] else { continue };
            let is_selected = selected.is_some() && l.change == selected;
            let base = settings.color_for(l.kind, dark);
            let block_color = base.map(|c| if is_selected { Settings::selected_color(c, dark) } else { c });

            let (lines, line_hs, marks, first_no) = if is_left {
                (&d.src, &l.left_line_h, &d.src_marks, d.src_line)
            } else {
                (&d.dst, &l.right_line_h, &d.dst_marks, d.dst_line)
            };

            // Block background (text area + tinted gutter).
            if let Some(c) = block_color {
                let block = Rect::from_min_size(Pos2::new(pane_rect.min.x, top), Vec2::new(pane_rect.width(), h));
                p.rect_filled(block, 0.0, c);
                let g = Rect::from_min_size(Pos2::new(pane_rect.min.x, top), Vec2::new(gutter_w, h));
                p.rect_filled(g, 0.0, crate::settings::scale(c, if dark { 1.15 } else { 0.9 }));
                if is_selected {
                    p.rect_stroke(
                        block.shrink(0.5),
                        0.0,
                        Stroke::new(1.0_f32, Settings::outline_color(c, dark)),
                        egui::StrokeKind::Inside,
                    );
                }
            }

            let mut y = top;
            for (li, line) in lines.iter().enumerate() {
                let lh = line_hs.get(li).copied().unwrap_or(row_h);
                if y + lh >= pane_rect.min.y && y <= pane_rect.max.y {
                    // line number
                    p.text(
                        Pos2::new(pane_rect.min.x + gutter_w - PAD / 2.0, y),
                        Align2::RIGHT_TOP,
                        (first_no + li).to_string(),
                        FontId::monospace(settings.font_size * 0.85),
                        weak_color,
                    );
                    let text = expand(line);
                    let line_color = if dark && base.is_some() { Color32::from_gray(238) } else { text_color };
                    let galley = ui.fonts(|f| f.layout(text, font.clone(), line_color, wrap_width));
                    content_w = content_w.max(galley.size().x);
                    let origin = Pos2::new(pane_rect.min.x + gutter_w + PAD - state.scroll_x, y);
                    // intra-line marks
                    if let (Some(c), Some(ranges)) = (base, marks.get(li)) {
                        if !ranges.is_empty() {
                            let mc = Settings::mark_color(c, dark);
                            draw_marks(&p, &galley, origin, ranges, mc);
                        }
                    }
                    p.galley(origin, galley, line_color);
                }
                y += lh;
            }
        }
    }
    state.content_w = content_w;

    // ---- connecting bands ----
    {
        let p = painter.with_clip_rect(connect);
        p.rect_filled(connect, 0.0, ui.visuals().panel_fill);
        for l in &layout {
            if l.is_hunk || l.kind == Kind::Unchanged {
                continue;
            }
            let lt = rect.min.y + l.yl - scroll_l;
            let lb = lt + l.hl;
            let rt = rect.min.y + l.yr - scroll_r;
            let rb = rt + l.hr;
            if lt.max(rt) > connect.max.y || lb.min(rb) < connect.min.y {
                continue;
            }
            let Some(c) = settings.color_for(l.kind, dark) else { continue };
            let is_selected = selected.is_some() && l.change == selected;
            let fill = if is_selected { Settings::selected_color(c, dark) } else { c };
            let pts = vec![
                Pos2::new(connect.min.x, lt),
                Pos2::new(connect.max.x, rt),
                Pos2::new(connect.max.x, rb.max(rt + 1.0)),
                Pos2::new(connect.min.x, lb.max(lt + 1.0)),
            ];
            let stroke = if is_selected {
                Stroke::new(1.0_f32, Settings::outline_color(c, dark))
            } else {
                Stroke::new(0.5_f32, crate::settings::scale(c, if dark { 1.3 } else { 0.85 }))
            };
            p.add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
    }

    // ---- scrollbar ----
    {
        let p = painter.with_clip_rect(scrollbar);
        p.rect_filled(scrollbar, 0.0, ui.visuals().panel_fill);
        let range = max_scroll - min_scroll;
        let thumb_h = (view_h * view_h / total_for_bar).clamp(20.0, view_h);
        let frac = if range > 0.0 { (state.scroll_y - min_scroll) / range } else { 0.0 };
        let thumb_y = scrollbar.min.y + frac * (view_h - thumb_h);
        let thumb = Rect::from_min_size(Pos2::new(scrollbar.min.x + 3.0, thumb_y), Vec2::new(SCROLLBAR_W - 6.0, thumb_h));
        let thumb_color = if dragging_scrollbar {
            ui.visuals().widgets.active.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };
        p.rect_filled(thumb, 3.0, thumb_color);
    }

    action
}

/// Paint the darker "changed characters" highlight behind the glyphs in `ranges` (char indices).
fn draw_marks(p: &egui::Painter, galley: &egui::Galley, origin: Pos2, ranges: &[(usize, usize)], color: Color32) {
    let mut idx = 0usize;
    for row in &galley.rows {
        let row_y0 = origin.y + row.rect.min.y;
        let row_h = row.rect.height();
        let mut run: Option<(f32, f32)> = None;
        for g in &row.glyphs {
            let in_range = ranges.iter().any(|&(a, b)| idx >= a && idx < b);
            let x0 = origin.x + g.pos.x;
            let x1 = x0 + g.advance_width;
            if in_range {
                run = Some(match run {
                    Some((a, _)) => (a, x1),
                    None => (x0, x1),
                });
            } else if let Some((a, b)) = run.take() {
                p.rect_filled(Rect::from_min_max(Pos2::new(a, row_y0), Pos2::new(b, row_y0 + row_h)), 2.0, color);
            }
            idx += 1;
        }
        if let Some((a, b)) = run {
            p.rect_filled(Rect::from_min_max(Pos2::new(a, row_y0), Pos2::new(b, row_y0 + row_h)), 2.0, color);
        }
        if row.ends_with_newline {
            idx += 1;
        }
    }
}

#[allow(dead_code)]
pub fn difference_summary(d: &Difference) -> String {
    d.description()
}
