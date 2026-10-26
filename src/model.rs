//! Diff data model, mirroring Kompare's DiffModel / DiffHunk / Difference.

use similar::{Algorithm, ChangeTag, TextDiff};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Kind {
    Unchanged,
    Change,
    Insert,
    Delete,
}

/// One block of lines: either unchanged context, or a change/insert/delete.
#[derive(Clone, Debug)]
pub struct Difference {
    pub kind: Kind,
    /// 1-based line number of the first source line (for inserts: the line after which text was inserted).
    pub src_line: usize,
    /// 1-based line number of the first destination line.
    pub dst_line: usize,
    pub src: Vec<String>,
    pub dst: Vec<String>,
    /// Per source line: character ranges (start, end) that differ from the paired destination line.
    pub src_marks: Vec<Vec<(usize, usize)>>,
    /// Per destination line: character ranges that differ from the paired source line.
    pub dst_marks: Vec<Vec<(usize, usize)>>,
}

impl Difference {
    pub fn new(kind: Kind, src_line: usize, dst_line: usize, src: Vec<String>, dst: Vec<String>) -> Self {
        let mut d = Difference {
            kind,
            src_line,
            dst_line,
            src_marks: vec![Vec::new(); src.len()],
            dst_marks: vec![Vec::new(); dst.len()],
            src,
            dst,
        };
        if kind == Kind::Change {
            d.compute_marks();
        }
        d
    }

    /// Kompare-style intra-line highlighting: pair the i-th source line with the i-th
    /// destination line and mark the character spans that differ, when the lines are similar enough.
    fn compute_marks(&mut self) {
        let n = self.src.len().min(self.dst.len());
        for i in 0..n {
            let a = &self.src[i];
            let b = &self.dst[i];
            let diff = TextDiff::configure().algorithm(Algorithm::Myers).diff_chars(a, b);
            if diff.ratio() < 0.35 {
                continue; // too different: highlight the whole line (no marks)
            }
            let mut a_pos = 0usize;
            let mut b_pos = 0usize;
            let mut a_marks = Vec::new();
            let mut b_marks = Vec::new();
            for change in diff.iter_all_changes() {
                let len = change.value().chars().count();
                match change.tag() {
                    ChangeTag::Equal => {
                        a_pos += len;
                        b_pos += len;
                    }
                    ChangeTag::Delete => {
                        push_merged(&mut a_marks, a_pos, a_pos + len);
                        a_pos += len;
                    }
                    ChangeTag::Insert => {
                        push_merged(&mut b_marks, b_pos, b_pos + len);
                        b_pos += len;
                    }
                }
            }
            self.src_marks[i] = a_marks;
            self.dst_marks[i] = b_marks;
        }
    }

    pub fn description(&self) -> String {
        fn plural(n: usize, word: &str) -> String {
            if n == 1 {
                format!("{n} {word}")
            } else {
                format!("{n} {word}s")
            }
        }
        match self.kind {
            Kind::Change => format!("Changed {}", plural(self.src.len(), "line")),
            Kind::Insert => format!("Inserted {}", plural(self.dst.len(), "line")),
            Kind::Delete => format!("Deleted {}", plural(self.src.len(), "line")),
            Kind::Unchanged => String::new(),
        }
    }
}

fn push_merged(marks: &mut Vec<(usize, usize)>, start: usize, end: usize) {
    if let Some(last) = marks.last_mut() {
        if last.1 == start {
            last.1 = end;
            return;
        }
    }
    marks.push((start, end));
}

#[derive(Clone, Debug)]
pub enum Segment {
    /// A hunk boundary, with the "@@ -a,b +c,d @@ context" header text.
    HunkHeader(String),
    Diff(Difference),
}

/// One compared file (one "model" in Kompare terms).
#[derive(Clone, Debug, Default)]
pub struct FileModel {
    pub source: String,
    pub destination: String,
    pub segments: Vec<Segment>,
    /// Indices into `segments` of the non-unchanged differences, in order.
    pub change_idx: Vec<usize>,
}

impl FileModel {
    pub fn new(source: impl Into<String>, destination: impl Into<String>) -> Self {
        FileModel {
            source: source.into(),
            destination: destination.into(),
            ..Default::default()
        }
    }

    pub fn push_hunk(&mut self, header: String) {
        self.segments.push(Segment::HunkHeader(header));
    }

    pub fn push_diff(&mut self, d: Difference) {
        if d.kind != Kind::Unchanged {
            self.change_idx.push(self.segments.len());
        }
        self.segments.push(Segment::Diff(d));
    }

    pub fn change_count(&self) -> usize {
        self.change_idx.len()
    }

    pub fn change(&self, k: usize) -> Option<&Difference> {
        match self.segments.get(*self.change_idx.get(k)?) {
            Some(Segment::Diff(d)) => Some(d),
            _ => None,
        }
    }

    pub fn hunk_count(&self) -> usize {
        self.segments.iter().filter(|s| matches!(s, Segment::HunkHeader(_))).count()
    }

    /// Display label: shared name when source == destination, else "a → b".
    #[allow(dead_code)]
    pub fn label(&self) -> String {
        if self.source == self.destination || self.source == "/dev/null" {
            self.destination.clone()
        } else if self.destination == "/dev/null" {
            self.source.clone()
        } else {
            format!("{} → {}", self.source, self.destination)
        }
    }
}

/// Strip the `a/`, `b/` prefixes git adds and any trailing tab-separated timestamp.
pub fn clean_name(raw: &str) -> String {
    let name = raw.split('\t').next().unwrap_or(raw).trim();
    for pre in ["a/", "b/"] {
        if let Some(rest) = name.strip_prefix(pre) {
            return rest.to_string();
        }
    }
    name.to_string()
}
