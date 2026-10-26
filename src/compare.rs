//! Direct comparison of two files (Kompare's "Compare Files" mode), using the `similar` crate
//! instead of shelling out to `diff`.

use std::path::Path;

use similar::{Algorithm, DiffOp, TextDiff};

use crate::model::{Difference, FileModel, Kind};

pub struct CompareResult {
    pub model: FileModel,
    /// Unified diff text, for the text view and "Save Diff".
    pub unified: String,
}

pub fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    if bytes.iter().take(8192).any(|&b| b == 0) {
        return Err(format!("{} looks like a binary file.", path.display()));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn compare_files(a: &Path, b: &Path, context_lines: usize) -> Result<CompareResult, String> {
    let old = read_text(a)?;
    let new = read_text(b)?;
    let diff = TextDiff::configure().algorithm(Algorithm::Patience).diff_lines(&old, &new);

    let old_lines: Vec<&str> = diff.old_slices().to_vec();
    let new_lines: Vec<&str> = diff.new_slices().to_vec();
    let strip = |s: &str| s.trim_end_matches(['\n', '\r']).to_string();

    let mut model = FileModel::new(a.display().to_string(), b.display().to_string());

    for group in diff.grouped_ops(context_lines) {
        if group.is_empty() {
            continue;
        }
        let (os, oe) = (group.first().unwrap().old_range().start, group.last().unwrap().old_range().end);
        let (ns, ne) = (group.first().unwrap().new_range().start, group.last().unwrap().new_range().end);
        model.push_hunk(format!("@@ -{},{} +{},{} @@", os + 1, oe - os, ns + 1, ne - ns));

        for op in group {
            match op {
                DiffOp::Equal { old_index, new_index, len } => {
                    let lines: Vec<String> = old_lines[old_index..old_index + len].iter().map(|s| strip(s)).collect();
                    model.push_diff(Difference::new(Kind::Unchanged, old_index + 1, new_index + 1, lines.clone(), lines));
                }
                DiffOp::Delete { old_index, old_len, new_index } => {
                    let lines = old_lines[old_index..old_index + old_len].iter().map(|s| strip(s)).collect();
                    model.push_diff(Difference::new(Kind::Delete, old_index + 1, new_index + 1, lines, Vec::new()));
                }
                DiffOp::Insert { old_index, new_index, new_len } => {
                    let lines = new_lines[new_index..new_index + new_len].iter().map(|s| strip(s)).collect();
                    model.push_diff(Difference::new(Kind::Insert, old_index + 1, new_index + 1, Vec::new(), lines));
                }
                DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    let src = old_lines[old_index..old_index + old_len].iter().map(|s| strip(s)).collect();
                    let dst = new_lines[new_index..new_index + new_len].iter().map(|s| strip(s)).collect();
                    model.push_diff(Difference::new(Kind::Change, old_index + 1, new_index + 1, src, dst));
                }
            }
        }
    }

    let unified = diff
        .unified_diff()
        .context_radius(context_lines)
        .header(&a.display().to_string(), &b.display().to_string())
        .to_string();

    Ok(CompareResult { model, unified })
}
