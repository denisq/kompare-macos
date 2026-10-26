//! Navigation sidebar: file list (for multi-file diffs) + changes list (Kompare's navigation part).

use egui_extras::{Column, TableBuilder};

use crate::model::FileModel;

pub enum NavAction {
    None,
    SelectFile(usize),
    SelectChange(usize),
}

const ROW_H: f32 = 24.0;

fn cell(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) {
    ui.add(egui::Label::new(text).selectable(false).truncate());
}

pub fn show(ui: &mut egui::Ui, files: &[FileModel], cur_file: usize, selected: Option<usize>) -> NavAction {
    let mut action = NavAction::None;
    // Rows must be clickable anywhere, so no text selection in this panel.
    ui.style_mut().interaction.selectable_labels = false;
    let avail_h = ui.available_height();

    if files.len() > 1 {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("Files ({})", files.len())).strong());
        ui.add_space(2.0);
        let files_h = ((files.len() as f32 + 1.0) * ROW_H + 8.0).min(avail_h * 0.4);
        ui.push_id("files_table", |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
                .max_scroll_height(files_h)
                .column(Column::remainder().clip(true))
                .header(ROW_H, |mut h| {
                    h.col(|ui| cell(ui, egui::RichText::new("Source → Destination").weak()));
                })
                .body(|body| {
                    body.rows(ROW_H, files.len(), |mut row| {
                        let i = row.index();
                        row.set_selected(i == cur_file);
                        row.col(|ui| cell(ui, files[i].label()));
                        if row.response().clicked() {
                            action = NavAction::SelectFile(i);
                        }
                    });
                });
        });
        ui.add_space(8.0);
        ui.separator();
    }

    let file = files.get(cur_file);
    let n = file.map(|f| f.change_count()).unwrap_or(0);
    ui.add_space(4.0);
    ui.label(egui::RichText::new(format!("Changes ({n})")).strong());
    ui.add_space(2.0);
    ui.push_id("changes_table", |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .sense(egui::Sense::click())
            .column(Column::auto().at_least(44.0))
            .column(Column::auto().at_least(44.0))
            .column(Column::remainder().clip(true))
            .header(ROW_H, |mut h| {
                h.col(|ui| cell(ui, egui::RichText::new("Src").weak()));
                h.col(|ui| cell(ui, egui::RichText::new("Dst").weak()));
                h.col(|ui| cell(ui, egui::RichText::new("Difference").weak()));
            })
            .body(|body| {
                body.rows(ROW_H, n, |mut row| {
                    let k = row.index();
                    row.set_selected(selected == Some(k));
                    if let Some(d) = file.and_then(|f| f.change(k)) {
                        row.col(|ui| cell(ui, d.src_line.to_string()));
                        row.col(|ui| cell(ui, d.dst_line.to_string()));
                        row.col(|ui| cell(ui, d.description()));
                    }
                    if row.response().clicked() {
                        action = NavAction::SelectChange(k);
                    }
                });
            });
    });

    action
}
