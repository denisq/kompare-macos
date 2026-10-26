use std::path::{Path, PathBuf};

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

use crate::compare;
use crate::macos;
use crate::model::FileModel;
use crate::nav::{self, NavAction};
use crate::settings::{Recent, Settings, ThemeChoice};
use crate::unified;
use crate::view::{self, ViewAction, ViewState};

const SETTINGS_KEY: &str = "kompare-settings";

#[derive(Clone, Debug)]
pub enum Mode {
    Empty,
    ShowingDiff(PathBuf),
    Comparing(PathBuf, PathBuf),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cmd {
    OpenDiff,
    CompareFiles,
    Refresh,
    Swap,
    Stats,
    PrevDiff,
    NextDiff,
    PrevFile,
    NextFile,
    ToggleTextView,
    ToggleNav,
    ToggleWrap,
    Prefs,
    Theme(ThemeChoice),
    Home,
}

struct Menus {
    _menu: Menu,
    items: Vec<(MenuId, Cmd)>,
    text_view: CheckMenuItem,
    nav: CheckMenuItem,
    wrap: CheckMenuItem,
    theme_items: Vec<(ThemeChoice, CheckMenuItem)>,
}

impl Menus {
    fn new(settings: &Settings) -> Self {
        let menu = Menu::new();
        let mut items = Vec::new();
        let acc = |m: Option<Modifiers>, c: Code| Some(Accelerator::new(m, c));
        let cmd = Some(Modifiers::META);
        let cmd_shift = Some(Modifiers::META | Modifiers::SHIFT);

        let mk = |items: &mut Vec<(MenuId, Cmd)>, text: &str, c: Cmd, a: Option<Accelerator>| -> MenuItem {
            let it = MenuItem::new(text, true, a);
            items.push((it.id().clone(), c));
            it
        };

        // App menu
        let prefs = mk(&mut items, "Preferences…", Cmd::Prefs, acc(cmd, Code::Comma));
        let app_menu = Submenu::new("Kompare", true);
        let _ = app_menu.append_items(&[
            &PredefinedMenuItem::about(
                Some("About Kompare"),
                Some(AboutMetadata {
                    name: Some("Kompare".into()),
                    version: Some(env!("CARGO_PKG_VERSION").into()),
                    comments: Some("A Rust reimplementation of KDE's Kompare diff viewer for macOS.".into()),
                    ..Default::default()
                }),
            ),
            &PredefinedMenuItem::separator(),
            &prefs,
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ]);

        // File menu (Kompare: Open Diff…, Compare Files…, Refresh Diff, Swap, Statistics)
        let open = mk(&mut items, "Open Diff…", Cmd::OpenDiff, acc(cmd, Code::KeyO));
        let compare = mk(&mut items, "Compare Files…", Cmd::CompareFiles, acc(cmd_shift, Code::KeyC));
        let refresh = mk(&mut items, "Refresh Diff", Cmd::Refresh, acc(cmd, Code::KeyR));
        let swap = mk(&mut items, "Swap Source with Destination", Cmd::Swap, None);
        let stats = mk(&mut items, "Show Statistics", Cmd::Stats, None);
        let close = mk(&mut items, "Close", Cmd::Home, acc(cmd, Code::KeyW));
        let file_menu = Submenu::new("File", true);
        let _ = file_menu.append_items(&[
            &open,
            &compare,
            &PredefinedMenuItem::separator(),
            &refresh,
            &swap,
            &stats,
            &PredefinedMenuItem::separator(),
            &close,
        ]);

        // Difference menu
        let prev_file = mk(&mut items, "Previous File", Cmd::PrevFile, acc(cmd, Code::PageUp));
        let next_file = mk(&mut items, "Next File", Cmd::NextFile, acc(cmd, Code::PageDown));
        let prev = mk(&mut items, "Previous Difference", Cmd::PrevDiff, acc(cmd, Code::ArrowUp));
        let next = mk(&mut items, "Next Difference", Cmd::NextDiff, acc(cmd, Code::ArrowDown));
        let diff_menu = Submenu::new("Difference", true);
        let _ = diff_menu.append_items(&[&prev_file, &next_file, &PredefinedMenuItem::separator(), &prev, &next]);

        // View menu
        let text_view = CheckMenuItem::new("Show Text View", true, settings.show_text_view, acc(cmd, Code::KeyT));
        items.push((text_view.id().clone(), Cmd::ToggleTextView));
        let nav_item = CheckMenuItem::new("Show Navigation", true, settings.show_nav, acc(cmd, Code::KeyN));
        items.push((nav_item.id().clone(), Cmd::ToggleNav));
        let wrap = CheckMenuItem::new("Wrap Long Lines", true, settings.wrap, acc(cmd_shift, Code::KeyW));
        items.push((wrap.id().clone(), Cmd::ToggleWrap));
        let theme_menu = Submenu::new("Theme", true);
        let mut theme_items = Vec::new();
        for (choice, label) in [(ThemeChoice::System, "System"), (ThemeChoice::Light, "Light"), (ThemeChoice::Dark, "Dark")] {
            let it = CheckMenuItem::new(label, true, settings.theme == choice, None);
            items.push((it.id().clone(), Cmd::Theme(choice)));
            let _ = theme_menu.append(&it);
            theme_items.push((choice, it));
        }
        let view_menu = Submenu::new("View", true);
        let _ = view_menu.append_items(&[&nav_item, &text_view, &PredefinedMenuItem::separator(), &wrap, &PredefinedMenuItem::separator(), &theme_menu]);

        let window_menu = Submenu::new("Window", true);
        let _ = window_menu.append_items(&[&PredefinedMenuItem::minimize(None), &PredefinedMenuItem::fullscreen(None)]);

        let _ = menu.append_items(&[&app_menu, &file_menu, &diff_menu, &view_menu, &window_menu]);

        #[cfg(target_os = "macos")]
        {
            menu.init_for_nsapp();
            window_menu.set_as_windows_menu_for_nsapp();
        }

        Menus { _menu: menu, items, text_view, nav: nav_item, wrap, theme_items }
    }

    fn sync_theme(&self, theme: ThemeChoice) {
        for (choice, item) in &self.theme_items {
            item.set_checked(*choice == theme);
        }
    }

    fn poll(&self) -> Vec<Cmd> {
        let mut cmds = Vec::new();
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some((_, c)) = self.items.iter().find(|(id, _)| *id == ev.id) {
                cmds.push(*c);
            }
        }
        cmds
    }
}

pub struct KompareApp {
    settings: Settings,
    files: Vec<FileModel>,
    cur_file: usize,
    selected: Option<usize>,
    mode: Mode,
    raw_text: String,
    view: ViewState,
    menus: Menus,
    error: Option<String>,
    info: Option<(String, String)>,
    show_prefs: bool,
    status: String,
    initial_args: Vec<PathBuf>,
    screenshot: Option<PathBuf>,
    frames: u32,
    shot_requested: bool,
    theme_dirty: bool,
    pending_cmd: Option<Cmd>,
    pending_recent: Option<Recent>,
}

impl KompareApp {
    pub fn new(cc: &eframe::CreationContext<'_>, args: Vec<PathBuf>, screenshot: Option<PathBuf>) -> Self {
        let settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, SETTINGS_KEY))
            .unwrap_or_default();
        let menus = Menus::new(&settings);
        let _ = macos::CTX.set(cc.egui_ctx.clone());
        // Debug aid: KOMPARE_THEME=dark|light forces a theme (the app follows the system otherwise).
        let env_theme = match std::env::var("KOMPARE_THEME").as_deref() {
            Ok("dark") => {
                cc.egui_ctx.set_theme(egui::Theme::Dark);
                true
            }
            Ok("light") => {
                cc.egui_ctx.set_theme(egui::Theme::Light);
                true
            }
            _ => false,
        };
        KompareApp {
            settings,
            files: Vec::new(),
            cur_file: 0,
            selected: None,
            mode: Mode::Empty,
            raw_text: String::new(),
            view: ViewState::default(),
            menus,
            error: None,
            info: None,
            show_prefs: false,
            status: String::new(),
            initial_args: args,
            screenshot,
            frames: 0,
            shot_requested: false,
            theme_dirty: !env_theme,
            pending_cmd: None,
            pending_recent: None,
        }
    }

    /// Debugging aid (`--screenshot out.ppm`): capture the window a few frames in, write it, quit.
    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        self.frames += 1;
        ctx.request_repaint();
        // Capture once content has arrived (argv, or a Finder open event), or after ~2s regardless.
        let ready = (!self.files.is_empty() && self.frames >= 6) || self.frames >= 120;
        if ready && !self.shot_requested {
            self.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let shots: Vec<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
                .collect()
        });
        if let Some(img) = shots.first() {
            let [w, h] = img.size;
            let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
            for px in &img.pixels {
                out.extend_from_slice(&[px.r(), px.g(), px.b()]);
            }
            if let Err(e) = std::fs::write(&path, out) {
                eprintln!("screenshot: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    // ---------------------------------------------------------------- loading

    fn open_paths(&mut self, paths: Vec<PathBuf>) {
        match paths.len() {
            0 => {}
            1 => self.open_diff(&paths[0]),
            _ => self.compare(&paths[0], &paths[1]),
        }
    }

    fn open_diff(&mut self, path: &Path) {
        match compare::read_text(path) {
            Err(e) => self.error = Some(e),
            Ok(text) => {
                let files = unified::parse(&text);
                if files.is_empty() {
                    self.error = Some(format!(
                        "{} does not contain a unified diff that Kompare can display.",
                        path.display()
                    ));
                    return;
                }
                self.raw_text = text;
                self.settings.push_recent(Recent::Diff(path.to_path_buf()));
                self.mode = Mode::ShowingDiff(path.to_path_buf());
                self.status = format!("Viewing diff output from {}", path.display());
                self.set_files(files);
            }
        }
    }

    fn compare(&mut self, a: &Path, b: &Path) {
        if a.is_dir() || b.is_dir() {
            self.error = Some("Folder comparison is not supported yet — compare two files or open a diff.".into());
            return;
        }
        match compare::compare_files(a, b, self.settings.context_lines) {
            Err(e) => self.error = Some(e),
            Ok(result) => {
                self.settings.push_recent(Recent::Compare(a.to_path_buf(), b.to_path_buf()));
                self.mode = Mode::Comparing(a.to_path_buf(), b.to_path_buf());
                self.status = format!("Comparing file {} with file {}", a.display(), b.display());
                if result.model.change_count() == 0 {
                    self.raw_text.clear();
                    self.set_files(Vec::new());
                    self.info = Some(("Compare Files".into(), "The files are identical.".into()));
                    return;
                }
                self.raw_text = result.unified;
                self.set_files(vec![result.model]);
            }
        }
    }

    fn set_files(&mut self, files: Vec<FileModel>) {
        self.files = files;
        self.cur_file = 0;
        self.selected = self.files.first().filter(|f| f.change_count() > 0).map(|_| 0);
        self.view = ViewState::default();
        self.view.scroll_to_selected = true;
    }

    fn refresh(&mut self) {
        match self.mode.clone() {
            Mode::Empty => {}
            Mode::ShowingDiff(p) => self.open_diff(&p),
            Mode::Comparing(a, b) => self.compare(&a, &b),
        }
    }

    fn swap(&mut self) {
        if let Mode::Comparing(a, b) = self.mode.clone() {
            self.compare(&b, &a);
        }
    }

    // ---------------------------------------------------------------- navigation

    fn select_file(&mut self, i: usize) {
        if i < self.files.len() {
            self.cur_file = i;
            self.selected = if self.files[i].change_count() > 0 { Some(0) } else { None };
            self.view.scroll_y = 0.0;
            self.view.scroll_to_selected = true;
        }
    }

    fn select_change(&mut self, k: usize, scroll: bool) {
        if let Some(f) = self.files.get(self.cur_file) {
            if k < f.change_count() {
                self.selected = Some(k);
                self.view.scroll_to_selected = scroll;
            }
        }
    }

    fn next_diff(&mut self) {
        let Some(f) = self.files.get(self.cur_file) else { return };
        match self.selected {
            Some(k) if k + 1 < f.change_count() => self.select_change(k + 1, true),
            None if f.change_count() > 0 => self.select_change(0, true),
            _ => {
                if self.cur_file + 1 < self.files.len() {
                    self.select_file(self.cur_file + 1);
                }
            }
        }
    }

    fn prev_diff(&mut self) {
        match self.selected {
            Some(k) if k > 0 => self.select_change(k - 1, true),
            _ => {
                if self.cur_file > 0 {
                    self.select_file(self.cur_file - 1);
                    let last = self.files[self.cur_file].change_count().saturating_sub(1);
                    self.select_change(last, true);
                }
            }
        }
    }

    fn run(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::OpenDiff => {
                if let Some(p) = rfd::FileDialog::new()
                    .set_title("Open Diff")
                    .add_filter("Patch files", &["diff", "patch", "dif"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.open_diff(&p);
                }
            }
            Cmd::CompareFiles => {
                let Some(a) = rfd::FileDialog::new().set_title("Compare Files — select the source file").pick_file() else {
                    return;
                };
                let Some(b) = rfd::FileDialog::new()
                    .set_title("Compare Files — select the destination file")
                    .set_directory(a.parent().unwrap_or(Path::new("/")))
                    .pick_file()
                else {
                    return;
                };
                self.compare(&a, &b);
            }
            Cmd::Refresh => self.refresh(),
            Cmd::Swap => self.swap(),
            Cmd::Stats => self.show_stats(),
            Cmd::PrevDiff => self.prev_diff(),
            Cmd::NextDiff => self.next_diff(),
            Cmd::PrevFile => {
                if self.cur_file > 0 {
                    self.select_file(self.cur_file - 1)
                }
            }
            Cmd::NextFile => {
                if self.cur_file + 1 < self.files.len() {
                    self.select_file(self.cur_file + 1)
                }
            }
            Cmd::ToggleTextView => {
                self.settings.show_text_view = !self.settings.show_text_view;
                self.menus.text_view.set_checked(self.settings.show_text_view);
            }
            Cmd::ToggleNav => {
                self.settings.show_nav = !self.settings.show_nav;
                self.menus.nav.set_checked(self.settings.show_nav);
            }
            Cmd::ToggleWrap => {
                self.settings.wrap = !self.settings.wrap;
                self.menus.wrap.set_checked(self.settings.wrap);
            }
            Cmd::Prefs => self.show_prefs = true,
            Cmd::Theme(t) => {
                self.settings.theme = t;
                self.menus.sync_theme(t);
                self.theme_dirty = true;
            }
            Cmd::Home => {
                self.files.clear();
                self.raw_text.clear();
                self.mode = Mode::Empty;
                self.selected = None;
                self.cur_file = 0;
                self.status.clear();
            }
        }
    }

    fn show_stats(&mut self) {
        let Some(f) = self.files.get(self.cur_file) else {
            self.info = Some((
                "Diff Statistics".into(),
                "No diff file, or no 2 files have been diffed. Therefore no stats are available.".into(),
            ));
            return;
        };
        let body = if self.files.len() == 1 {
            format!(
                "Statistics:\n\nOld file: {}\nNew file: {}\n\nFormat: Unified\nNumber of hunks: {}\nNumber of differences: {}",
                f.source,
                f.destination,
                f.hunk_count(),
                f.change_count()
            )
        } else {
            format!(
                "Statistics:\n\nNumber of files in diff file: {}\nFormat: Unified\n\nCurrent old file: {}\nCurrent new file: {}\n\nNumber of hunks: {}\nNumber of differences: {}",
                self.files.len(),
                f.source,
                f.destination,
                f.hunk_count(),
                f.change_count()
            )
        };
        self.info = Some(("Diff Statistics".into(), body));
    }

    fn window_title(&self) -> String {
        match &self.mode {
            Mode::Empty => "Kompare".to_string(),
            Mode::ShowingDiff(p) => format!("{} — Kompare", p.file_name().unwrap_or_default().to_string_lossy()),
            Mode::Comparing(a, b) => format!(
                "{} -- {} — Kompare",
                a.file_name().unwrap_or_default().to_string_lossy(),
                b.file_name().unwrap_or_default().to_string_lossy()
            ),
        }
    }

    // ---------------------------------------------------------------- UI pieces

    /// Welcome screen shown when nothing is loaded. Returns a command if a button was clicked.
    fn start_screen(&mut self, ui: &mut egui::Ui) -> Option<Cmd> {
        let mut cmd = None;
        let dark = ui.visuals().dark_mode;
        ui.painter().rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        let avail = ui.available_size();
        let card_w = 520.0_f32.min(avail.x - 40.0);
        let top_pad = (avail.y * 0.12).max(16.0);

        ui.vertical_centered(|ui| {
            ui.add_space(top_pad);
            ui.set_max_width(card_w);

            ui.label(egui::RichText::new("Kompare").size(30.0).strong());
            ui.label(egui::RichText::new("View the differences between two files, or open a diff").weak().size(14.0));
            ui.add_space(22.0);

            let big = |label: &str, hint: &str| {
                egui::Button::new(
                    egui::RichText::new(label).size(16.0).strong(),
                )
                .min_size(egui::vec2(card_w, 52.0))
                .shortcut_text(hint)
            };
            if ui.add(big("Open Diff…", "⌘O")).clicked() {
                cmd = Some(Cmd::OpenDiff);
            }
            ui.add_space(8.0);
            if ui.add(big("Compare Two Files…", "Shift ⌘C")).clicked() {
                cmd = Some(Cmd::CompareFiles);
            }
            ui.add_space(10.0);
            ui.label(egui::RichText::new("or drop a .diff / .patch file (or two files to compare) onto this window").weak());

            let recents: Vec<Recent> = self.settings.recents.clone();
            if !recents.is_empty() {
                ui.add_space(26.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Recent").strong().size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Clear").clicked() {
                            self.settings.recents.clear();
                        }
                    });
                });
                ui.add_space(4.0);
                let row_bg = if dark { egui::Color32::from_gray(38) } else { egui::Color32::from_gray(246) };
                egui::ScrollArea::vertical().max_height(avail.y * 0.45).show(ui, |ui| {
                    for r in &recents {
                        let exists = r.exists();
                        let resp = egui::Frame::NONE
                            .fill(row_bg)
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_width(card_w - 24.0);
                                ui.horizontal(|ui| {
                                    let icon = match r {
                                        Recent::Diff(_) => "📄",
                                        Recent::Compare(..) => "📑",
                                    };
                                    ui.label(egui::RichText::new(icon).size(18.0));
                                    ui.vertical(|ui| {
                                        let mut title = egui::RichText::new(r.title()).size(14.0);
                                        if !exists {
                                            title = title.strikethrough().weak();
                                        }
                                        ui.label(title);
                                        ui.label(egui::RichText::new(r.subtitle()).weak().size(11.0));
                                    });
                                });
                            })
                            .response
                            .interact(egui::Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            if exists {
                                self.pending_recent = Some(r.clone());
                            } else {
                                self.settings.recents.retain(|x| x != r);
                            }
                        }
                        ui.add_space(4.0);
                    }
                });
            }
        });
        cmd
    }

    fn prefs_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_prefs;
        egui::Window::new("Preferences")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Appearance");
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    for (choice, label) in [(ThemeChoice::System, "System"), (ThemeChoice::Light, "Light"), (ThemeChoice::Dark, "Dark")] {
                        if ui.radio(self.settings.theme == choice, label).clicked() && self.settings.theme != choice {
                            self.settings.theme = choice;
                            self.menus.sync_theme(choice);
                            self.theme_dirty = true;
                        }
                    }
                });
                ui.separator();
                ui.heading("Colors");
                egui::Grid::new("colors").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Removed color:");
                    ui.color_edit_button_srgba(&mut self.settings.remove_color);
                    ui.end_row();
                    ui.label("Changed color:");
                    ui.color_edit_button_srgba(&mut self.settings.change_color);
                    ui.end_row();
                    ui.label("Added color:");
                    ui.color_edit_button_srgba(&mut self.settings.add_color);
                    ui.end_row();
                });
                if ui.button("Restore Kompare defaults").clicked() {
                    let d = Settings::default();
                    self.settings.remove_color = d.remove_color;
                    self.settings.change_color = d.change_color;
                    self.settings.add_color = d.add_color;
                    self.settings.applied_color = d.applied_color;
                }
                ui.separator();
                ui.heading("Text");
                egui::Grid::new("text").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Font size:");
                    ui.add(egui::Slider::new(&mut self.settings.font_size, 8.0..=24.0).step_by(1.0));
                    ui.end_row();
                    ui.label("Tab width:");
                    ui.add(egui::Slider::new(&mut self.settings.tab_width, 1..=16));
                    ui.end_row();
                    ui.label("Wrap long lines:");
                    if ui.checkbox(&mut self.settings.wrap, "").changed() {
                        self.menus.wrap.set_checked(self.settings.wrap);
                    }
                    ui.end_row();
                });
                ui.separator();
                ui.heading("Diff");
                egui::Grid::new("diff").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Lines of context (Compare Files):");
                    ui.add(egui::Slider::new(&mut self.settings.context_lines, 0..=20));
                    ui.end_row();
                });
            });
        self.show_prefs = open;
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(msg) = self.error.clone() {
            let mut close = false;
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_max_width(480.0);
                    ui.label(msg);
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
                            close = true;
                        }
                    });
                });
            if close {
                self.error = None;
            }
        }
        if let Some((title, msg)) = self.info.clone() {
            let mut close = false;
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_max_width(480.0);
                    ui.label(msg);
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
                            close = true;
                        }
                    });
                });
            if close {
                self.info = None;
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) -> Option<Cmd> {
        let mut cmd = None;
        ui.style_mut().interaction.selectable_labels = false;
        let n = self.files.get(self.cur_file).map(|f| f.change_count()).unwrap_or(0);
        let n_files = self.files.len();
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if ui.button("Home").on_hover_text("Close this diff and return to the start screen (⌘W)").clicked() {
                cmd = Some(Cmd::Home);
            }
            if ui.button("Open…").on_hover_text("Open a diff file (⌘O)").clicked() {
                cmd = Some(Cmd::OpenDiff);
            }
            if ui.button("Compare…").on_hover_text("Compare two files (Shift ⌘C)").clicked() {
                cmd = Some(Cmd::CompareFiles);
            }
            ui.separator();

            let can_prev = self.selected.map(|k| k > 0).unwrap_or(false) || self.cur_file > 0;
            let can_next = self.selected.map(|k| k + 1 < n).unwrap_or(n > 0) || self.cur_file + 1 < n_files;
            if ui.add_enabled(can_prev, egui::Button::new("Prev")).on_hover_text("Previous difference (⌘↑)").clicked() {
                cmd = Some(Cmd::PrevDiff);
            }
            let counter = match self.selected {
                Some(k) => format!("{} / {} differences", k + 1, n),
                None => format!("{n} differences"),
            };
            ui.add(egui::Label::new(egui::RichText::new(counter).monospace()).selectable(false))
                .on_hover_text(&self.status);
            if ui.add_enabled(can_next, egui::Button::new("Next")).on_hover_text("Next difference (⌘↓)").clicked() {
                cmd = Some(Cmd::NextDiff);
            }
            if n_files > 1 {
                ui.separator();
                if ui.add_enabled(self.cur_file > 0, egui::Button::new("« File")).on_hover_text("Previous file (⌘PgUp)").clicked() {
                    cmd = Some(Cmd::PrevFile);
                }
                ui.add(egui::Label::new(egui::RichText::new(format!("{} / {} files", self.cur_file + 1, n_files)).monospace()).selectable(false));
                if ui.add_enabled(self.cur_file + 1 < n_files, egui::Button::new("File »")).on_hover_text("Next file (⌘PgDn)").clicked() {
                    cmd = Some(Cmd::NextFile);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.toggle_value(&mut self.settings.show_text_view, "Text").on_hover_text("Show the raw diff text (⌘T)").changed() {
                    self.menus.text_view.set_checked(self.settings.show_text_view);
                }
                if ui.toggle_value(&mut self.settings.wrap, "Wrap").on_hover_text("Wrap long lines (Shift ⌘W)").changed() {
                    self.menus.wrap.set_checked(self.settings.wrap);
                }
                if ui.toggle_value(&mut self.settings.show_nav, "Sidebar").on_hover_text("Show the navigation sidebar (⌘N)").changed() {
                    self.menus.nav.set_checked(self.settings.show_nav);
                }
            });
        });
        ui.add_space(2.0);
        cmd
    }
}

impl eframe::App for KompareApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SETTINGS_KEY, &self.settings);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Files from argv (first frame) and from Finder / `open` (any time).
        if !self.initial_args.is_empty() {
            let args = std::mem::take(&mut self.initial_args);
            self.open_paths(args);
        }
        let pending = macos::take_pending();
        if !pending.is_empty() {
            self.open_paths(pending);
        }
        // Drag & drop onto the window.
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.open_paths(dropped);
        }

        for cmd in self.menus.poll() {
            self.run(cmd);
        }
        if self.theme_dirty {
            self.settings.theme.apply(ctx);
            self.theme_dirty = false;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));

        let has_files = !self.files.is_empty();

        if has_files {
            let mut tb_cmd = None;
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                tb_cmd = self.toolbar(ui);
            });
            if let Some(c) = tb_cmd {
                self.run(c);
            }
        }

        if has_files && self.settings.show_text_view && !self.raw_text.is_empty() {
            egui::TopBottomPanel::bottom("textview")
                .resizable(true)
                .default_height(180.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| {
                        let mut text: &str = &self.raw_text;
                        ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .desired_width(f32::INFINITY),
                        );
                    });
                });
        }

        if has_files && self.settings.show_nav {
            let mut nav_action = NavAction::None;
            egui::SidePanel::left("nav")
                .resizable(true)
                .default_width(300.0)
                .min_width(180.0)
                .show(ctx, |ui| {
                    nav_action = nav::show(ui, &self.files, self.cur_file, self.selected);
                });
            match nav_action {
                NavAction::None => {}
                NavAction::SelectFile(i) => self.select_file(i),
                NavAction::SelectChange(k) => self.select_change(k, true),
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| match self.files.get(self.cur_file) {
                Some(file) => {
                    let action = view::show(ui, file, self.selected, &self.settings, &mut self.view);
                    if let ViewAction::Select(k) = action {
                        self.select_change(k, false);
                    }
                }
                None => {
                    if let Some(cmd) = self.start_screen(ui) {
                        self.pending_cmd = Some(cmd);
                    }
                }
            });

        if let Some(cmd) = self.pending_cmd.take() {
            self.run(cmd);
        }
        if let Some(r) = self.pending_recent.take() {
            match r {
                Recent::Diff(p) => self.open_diff(&p),
                Recent::Compare(a, b) => self.compare(&a, &b),
            }
        }

        if self.show_prefs {
            self.prefs_window(ctx);
        }
        self.dialogs(ctx);
        self.handle_screenshot(ctx);
    }
}
