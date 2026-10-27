//! Direct comparison of two files (Kompare's "Compare Files" mode), using the `similar` crate
//! instead of shelling out to `diff`.

use std::path::Path;

use similar::{capture_diff_slices, group_diff_ops, Algorithm, DiffOp};

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

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub context: usize,
    pub ignore_ws: bool,     // diff -b: ignore changes in the amount of whitespace
    pub ignore_case: bool,   // diff -i
    pub ignore_blank: bool,  // diff -B: ignore inserted/deleted blank lines
}

impl Default for Options {
    fn default() -> Self {
        Options { context: 3, ignore_ws: false, ignore_case: false, ignore_blank: false }
    }
}

pub fn compare_files(a: &Path, b: &Path, o: &Options) -> Result<CompareResult, String> {
    let old = read_text(a)?;
    let new = read_text(b)?;
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // Compare normalized keys, display the original lines.
    let key = |s: &str| -> String {
        let mut t = if o.ignore_ws { s.split_whitespace().collect::<Vec<_>>().join(" ") } else { s.to_string() };
        if o.ignore_case {
            t = t.to_lowercase();
        }
        t
    };
    let ok: Vec<String> = old_lines.iter().map(|s| key(s)).collect();
    let nk: Vec<String> = new_lines.iter().map(|s| key(s)).collect();
    let ops = capture_diff_slices(Algorithm::Patience, &ok, &nk);
    let all_blank = |lines: &[&str]| lines.iter().all(|l| l.trim().is_empty());

    let mut model = FileModel::new(a.display().to_string(), b.display().to_string());
    let mut unified = format!("--- {}\n+++ {}\n", a.display(), b.display());
    let owned = |v: &[&str]| -> Vec<String> { v.iter().map(|s| s.to_string()).collect() };

    for group in group_diff_ops(ops, o.context) {
        if group.is_empty() {
            continue;
        }
        let (os, oe) = (group.first().unwrap().old_range().start, group.last().unwrap().old_range().end);
        let (ns, ne) = (group.first().unwrap().new_range().start, group.last().unwrap().new_range().end);
        let header = format!("@@ -{},{} +{},{} @@", os + 1, oe - os, ns + 1, ne - ns);
        model.push_hunk(header.clone());
        unified.push_str(&header);
        unified.push('\n');

        for op in group {
            match op {
                DiffOp::Equal { old_index, new_index, len } => {
                    let lines = owned(&old_lines[old_index..old_index + len]);
                    for l in &lines { unified.push(' '); unified.push_str(l); unified.push('\n'); }
                    model.push_diff(Difference::new(Kind::Unchanged, old_index + 1, new_index + 1, lines.clone(), lines));
                }
                DiffOp::Delete { old_index, old_len, new_index } => {
                    let src = &old_lines[old_index..old_index + old_len];
                    for l in src { unified.push('-'); unified.push_str(l); unified.push('\n'); }
                    let kind = if o.ignore_blank && all_blank(src) { Kind::Unchanged } else { Kind::Delete };
                    model.push_diff(Difference::new(kind, old_index + 1, new_index + 1, owned(src), Vec::new()));
                }
                DiffOp::Insert { old_index, new_index, new_len } => {
                    let dst = &new_lines[new_index..new_index + new_len];
                    for l in dst { unified.push('+'); unified.push_str(l); unified.push('\n'); }
                    let kind = if o.ignore_blank && all_blank(dst) { Kind::Unchanged } else { Kind::Insert };
                    model.push_diff(Difference::new(kind, old_index + 1, new_index + 1, Vec::new(), owned(dst)));
                }
                DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    let src = &old_lines[old_index..old_index + old_len];
                    let dst = &new_lines[new_index..new_index + new_len];
                    for l in src { unified.push('-'); unified.push_str(l); unified.push('\n'); }
                    for l in dst { unified.push('+'); unified.push_str(l); unified.push('\n'); }
                    model.push_diff(Difference::new(Kind::Change, old_index + 1, new_index + 1, owned(src), owned(dst)));
                }
            }
        }
    }

    Ok(CompareResult { model, unified })
}
