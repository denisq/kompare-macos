#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Kompare web-UI variant: the same Rust diff core, with the view rendered as HTML/CSS
//! inside macOS's built-in WebView (Tauri). No Node/npm: the UI is a static `ui/` folder.

#![allow(dead_code)]

mod compare;
mod model;
mod unified;

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{Emitter, Manager, RunEvent, State};

/// Files handed to us before the webview asked for them (argv, or Finder open events at launch).
struct Pending(Mutex<Vec<String>>);

#[derive(Serialize)]
struct Loaded {
    kind: String,
    title: String,
    source: String,
    destination: String,
    files: Vec<model::FileModel>,
    raw: String,
}

#[tauri::command]
fn open_diff(path: String) -> Result<Loaded, String> {
    let p = Path::new(&path);
    let text = compare::read_text(p)?;
    let files = unified::parse(&text);
    if files.is_empty() {
        return Err(format!("{} does not contain a unified diff that Kompare can display.", p.display()));
    }
    Ok(Loaded {
        kind: "diff".into(),
        title: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        source: path.clone(),
        destination: String::new(),
        files,
        raw: text,
    })
}

#[tauri::command]
fn compare_files(a: String, b: String, context: usize, ignore_ws: bool, ignore_case: bool, ignore_blank: bool) -> Result<Loaded, String> {
    let (pa, pb) = (Path::new(&a), Path::new(&b));
    if pa.is_dir() || pb.is_dir() {
        return Err("Folder comparison is not supported yet — compare two files or open a diff.".into());
    }
    let opts = compare::Options { context, ignore_ws, ignore_case, ignore_blank };
    let r = compare::compare_files(pa, pb, &opts)?;
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Loaded {
        kind: "compare".into(),
        title: format!("{} ↔ {}", name(pa), name(pb)),
        source: a.clone(),
        destination: b.clone(),
        files: vec![r.model],
        raw: r.unified,
    })
}

#[tauri::command]
async fn pick_file(title: String, patch_filter: bool) -> Option<String> {
    let mut d = rfd::AsyncFileDialog::new().set_title(&title);
    if patch_filter {
        d = d.add_filter("Patch files", &["diff", "patch", "dif"]).add_filter("All files", &["*"]);
    }
    d.pick_file().await.map(|f| f.path().display().to_string())
}

#[tauri::command]
fn take_pending(state: State<Pending>) -> Vec<String> {
    std::mem::take(&mut *state.0.lock().unwrap())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with("-psn")).collect();

    tauri::Builder::default()
        .manage(Pending(Mutex::new(args)))
        .invoke_handler(tauri::generate_handler![open_diff, compare_files, pick_file, take_pending])
        .setup(|app| {
            let handle = app.handle();
            let open = MenuItemBuilder::with_id("open", "Open Diff…").accelerator("CmdOrCtrl+O").build(handle)?;
            let compare = MenuItemBuilder::with_id("compare", "Compare Files…").accelerator("CmdOrCtrl+Shift+C").build(handle)?;
            let close = MenuItemBuilder::with_id("home", "Close").accelerator("CmdOrCtrl+W").build(handle)?;
            let prev = MenuItemBuilder::with_id("prev", "Previous Difference").accelerator("CmdOrCtrl+Up").build(handle)?;
            let next = MenuItemBuilder::with_id("next", "Next Difference").accelerator("CmdOrCtrl+Down").build(handle)?;
            let prev_file = MenuItemBuilder::with_id("prev_file", "Previous File").accelerator("CmdOrCtrl+PageUp").build(handle)?;
            let next_file = MenuItemBuilder::with_id("next_file", "Next File").accelerator("CmdOrCtrl+PageDown").build(handle)?;
            let sidebar = MenuItemBuilder::with_id("sidebar", "Toggle Sidebar").accelerator("CmdOrCtrl+N").build(handle)?;
            let wrap = MenuItemBuilder::with_id("wrap", "Toggle Wrap").accelerator("CmdOrCtrl+Shift+W").build(handle)?;
            let find = MenuItemBuilder::with_id("find", "Find…").accelerator("CmdOrCtrl+F").build(handle)?;
            let options = MenuItemBuilder::with_id("options", "Diff Options…").accelerator("CmdOrCtrl+,").build(handle)?;

            let app_menu = SubmenuBuilder::new(handle, "Kompare")
                .about(None)
                .separator()
                .hide()
                .hide_others()
                .separator()
                .quit()
                .build()?;
            let file_menu = SubmenuBuilder::new(handle, "File")
                .items(&[&open, &compare, &PredefinedMenuItem::separator(handle)?, &close])
                .build()?;
            let diff_menu = SubmenuBuilder::new(handle, "Difference")
                .items(&[&prev_file, &next_file, &PredefinedMenuItem::separator(handle)?, &prev, &next])
                .build()?;
            let view_menu = SubmenuBuilder::new(handle, "View")
                .items(&[&sidebar, &wrap, &PredefinedMenuItem::separator(handle)?, &find, &options])
                .build()?;
            let window_menu = SubmenuBuilder::new(handle, "Window").minimize().fullscreen().build()?;
            let menu = MenuBuilder::new(handle)
                .items(&[&app_menu, &file_menu, &diff_menu, &view_menu, &window_menu])
                .build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu", event.id().0.clone());
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Kompare")
        .run(|app, event| {
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let RunEvent::Opened { urls } = event {
                let paths: Vec<String> = urls
                    .iter()
                    .filter_map(|u| u.to_file_path().ok())
                    .map(|p| p.display().to_string())
                    .collect();
                app.state::<Pending>().0.lock().unwrap().extend(paths.clone());
                let _ = app.emit("files-opened", paths);
            }
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            let _ = (app, event);
        });
}
