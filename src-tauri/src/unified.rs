//! Parser for unified diff output (`diff -u`, `git diff`, multi-file patches).

use crate::model::{clean_name, Difference, FileModel, Kind};

struct HunkHeader {
    old_start: usize,
    old_len: usize,
    new_start: usize,
    new_len: usize,
    context: String,
}

fn parse_hunk_header(line: &str) -> Option<HunkHeader> {
    // @@ -a[,b] +c[,d] @@ optional context
    let rest = line.strip_prefix("@@ -")?;
    let (ranges, context) = match rest.find(" @@") {
        Some(pos) => (&rest[..pos], rest[pos + 3..].trim().to_string()),
        None => return None,
    };
    let mut parts = ranges.split(" +");
    let old = parts.next()?;
    let new = parts.next()?;
    let parse_range = |s: &str| -> Option<(usize, usize)> {
        let mut it = s.split(',');
        let start = it.next()?.trim().parse().ok()?;
        let len = match it.next() {
            Some(l) => l.trim().parse().ok()?,
            None => 1,
        };
        Some((start, len))
    };
    let (old_start, old_len) = parse_range(old)?;
    let (new_start, new_len) = parse_range(new)?;
    Some(HunkHeader { old_start, old_len, new_start, new_len, context })
}

/// Parse unified diff text into file models. Returns an empty vector when nothing parseable was found.
pub fn parse(text: &str) -> Vec<FileModel> {
    let lines: Vec<&str> = text.lines().collect();
    let mut files: Vec<FileModel> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("--- ") && i + 1 < lines.len() && lines[i + 1].starts_with("+++ ") {
            let src = clean_name(&line[4..]);
            let dst = clean_name(&lines[i + 1][4..]);
            files.push(FileModel::new(src, dst));
            i += 2;
            continue;
        }

        let Some(h) = parse_hunk_header(line) else {
            i += 1;
            continue;
        };
        if files.is_empty() {
            files.push(FileModel::new("unknown", "unknown"));
        }
        let file = files.last_mut().unwrap();
        let header_text = if h.context.is_empty() {
            format!("@@ -{},{} +{},{} @@", h.old_start, h.old_len, h.new_start, h.new_len)
        } else {
            format!("@@ -{},{} +{},{} @@ {}", h.old_start, h.old_len, h.new_start, h.new_len, h.context)
        };
        file.push_hunk(header_text);
        i += 1;

        let mut a_no = h.old_start;
        let mut b_no = h.new_start;
        let mut a_left = h.old_len as isize;
        let mut b_left = h.new_len as isize;

        // Pending runs of removed / added lines, paired into one Difference when flushed.
        let mut dels: Vec<String> = Vec::new();
        let mut inss: Vec<String> = Vec::new();
        let mut run_a_no = a_no;
        let mut run_b_no = b_no;
        let mut ctx: Vec<String> = Vec::new();
        let mut ctx_a_no = a_no;
        let mut ctx_b_no = b_no;

        fn flush_ctx(file: &mut FileModel, ctx: &mut Vec<String>, a: usize, b: usize) {
            if !ctx.is_empty() {
                let lines = std::mem::take(ctx);
                file.push_diff(Difference::new(Kind::Unchanged, a, b, lines.clone(), lines));
            }
        }
        fn flush_change(file: &mut FileModel, dels: &mut Vec<String>, inss: &mut Vec<String>, a: usize, b: usize) {
            if dels.is_empty() && inss.is_empty() {
                return;
            }
            let kind = if !dels.is_empty() && !inss.is_empty() {
                Kind::Change
            } else if !dels.is_empty() {
                Kind::Delete
            } else {
                Kind::Insert
            };
            file.push_diff(Difference::new(kind, a, b, std::mem::take(dels), std::mem::take(inss)));
        }

        while i < lines.len() && (a_left > 0 || b_left > 0) {
            let body = lines[i];
            i += 1;
            if body.starts_with('\\') {
                continue; // "\ No newline at end of file"
            }
            if let Some(t) = body.strip_prefix('-') {
                if !ctx.is_empty() {
                    flush_ctx(file, &mut ctx, ctx_a_no, ctx_b_no);
                }
                if dels.is_empty() && inss.is_empty() {
                    run_a_no = a_no;
                    run_b_no = b_no;
                }
                dels.push(t.to_string());
                a_no += 1;
                a_left -= 1;
            } else if let Some(t) = body.strip_prefix('+') {
                if !ctx.is_empty() {
                    flush_ctx(file, &mut ctx, ctx_a_no, ctx_b_no);
                }
                if dels.is_empty() && inss.is_empty() {
                    run_a_no = a_no;
                    run_b_no = b_no;
                }
                inss.push(t.to_string());
                b_no += 1;
                b_left -= 1;
            } else {
                // context line (a blank line counts as an empty context line)
                flush_change(file, &mut dels, &mut inss, run_a_no, run_b_no);
                if ctx.is_empty() {
                    ctx_a_no = a_no;
                    ctx_b_no = b_no;
                }
                let t = body.strip_prefix(' ').unwrap_or(body);
                ctx.push(t.to_string());
                a_no += 1;
                b_no += 1;
                a_left -= 1;
                b_left -= 1;
            }
        }
        flush_change(file, &mut dels, &mut inss, run_a_no, run_b_no);
        flush_ctx(file, &mut ctx, ctx_a_no, ctx_b_no);
    }

    files.retain(|f| !f.segments.is_empty());
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_hunk() {
        let text = "--- a.txt\n+++ b.txt\n@@ -1,3 +1,3 @@\n keep\n-old\n+new\n tail\n";
        let files = parse(text);
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.change_count(), 1);
        let d = f.change(0).unwrap();
        assert_eq!(d.kind, Kind::Change);
        assert_eq!(d.src_line, 2);
        assert_eq!(d.src, vec!["old"]);
        assert_eq!(d.dst, vec!["new"]);
    }

    #[test]
    fn parses_multi_file_git_diff() {
        let text = "diff --git a/x.md b/x.md\n--- a/x.md\n+++ b/x.md\n@@ -1 +1 @@\n-a\n+b\ndiff --git a/y.md b/y.md\n--- a/y.md\n+++ b/y.md\n@@ -1,2 +1,3 @@\n a\n+b\n c\n";
        let files = parse(text);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].source, "x.md");
        assert_eq!(files[1].change(0).unwrap().kind, Kind::Insert);
    }
}
