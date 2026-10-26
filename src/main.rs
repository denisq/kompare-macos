#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod compare;
mod macos;
mod model;
mod nav;
mod settings;
mod unified;
mod view;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut screenshot: Option<PathBuf> = None;
    let mut args: Vec<PathBuf> = Vec::new();
    let mut it = std::env::args_os().skip(1);
    while let Some(a) = it.next() {
        let s = a.to_string_lossy();
        if s == "--screenshot" {
            screenshot = it.next().map(PathBuf::from); // debugging aid: dump first frame to a .ppm and exit
        } else if !s.starts_with("-psn") {
            // (-psn is the legacy Finder process-serial argument)
            args.push(PathBuf::from(a));
        }
    }

    macos::install_open_files_hook();

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../bundle/Kompare.iconset/icon_256x256.png"))
        .expect("embedded icon");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon)
            .with_inner_size([1150.0, 780.0])
            .with_min_inner_size([640.0, 400.0])
            .with_title("Kompare")
            .with_app_id("com.quentin.kompare"),
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "Kompare",
        options,
        Box::new(move |cc| Ok(Box::new(app::KompareApp::new(cc, args, screenshot)))),
    )
}
