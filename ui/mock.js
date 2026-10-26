// Browser-only preview data (same JSON shape the Rust core returns). Open index.html#mock.
window.MOCK = {
  kind: "diff",
  title: "Q2_report_iteration.diff",
  source: "/Users/quentin/Desktop/Q2_report_iteration.diff",
  destination: "",
  raw: "--- Q2_report_v1.md\n+++ Q2_report_v2.md\n@@ -2,13 +2,14 @@\n \n ## Summary\n \n-We processed 14 contracts ...\n+We processed 17 contracts ...\n",
  files: [{
    source: "Q2_report_v1.md", destination: "Q2_report_v2.md",
    change_idx: [3, 7, 9, 13],
    segments: [
      { HunkHeader: "@@ -2,13 +2,14 @@" },
      { Diff: { kind: "Unchanged", src_line: 2, dst_line: 2, src: ["", "## Summary", ""], dst: ["", "## Summary", ""], src_marks: [[],[],[]], dst_marks: [[],[],[]] } },
      { HunkHeader: "" },
      { Diff: { kind: "Change", src_line: 5, dst_line: 5,
        src: ["We processed 14 contracts this quarter, of which 11 were converted to Sales Orders without manual intervention. The remaining three required customer deduplication in Odoo before the SO could be created."],
        dst: ["We processed 17 contracts this quarter, of which 14 were converted to Sales Orders without manual intervention. The remaining three required customer deduplication in Odoo before the SO could be created. Average processing time dropped to 6 minutes per contract."],
        src_marks: [[[14,15],[45,46]]], dst_marks: [[[14,15],[45,46],[198,255]]] } },
      { Diff: { kind: "Unchanged", src_line: 6, dst_line: 6, src: ["", "## Open issues", ""], dst: ["", "## Open issues", ""], src_marks: [[],[],[]], dst_marks: [[],[],[]] } },
      { HunkHeader: "" },
      { HunkHeader: "" },
      { Diff: { kind: "Delete", src_line: 9, dst_line: 9, src: ["- VAT verification fails silently for Greek customers"], dst: [], src_marks: [[]], dst_marks: [] } },
      { Diff: { kind: "Unchanged", src_line: 10, dst_line: 9, src: ["- Break-clause detection misses German-language contracts"], dst: ["- Break-clause detection misses German-language contracts"], src_marks: [[]], dst_marks: [[]] } },
      { Diff: { kind: "Insert", src_line: 11, dst_line: 10, src: [], dst: ["- Quarterly instalment descriptions need the Y{n} - Q{n} suffix"], src_marks: [], dst_marks: [[]] } },
      { Diff: { kind: "Unchanged", src_line: 11, dst_line: 11, src: ["", "## Next steps", "", "Review the extraction prompt for German contracts."], dst: ["", "## Next steps", "", "Review the extraction prompt for German contracts."], src_marks: [[],[],[],[]], dst_marks: [[],[],[],[]] } },
      { HunkHeader: "" },
      { HunkHeader: "" },
      { Diff: { kind: "Insert", src_line: 15, dst_line: 15, src: [], dst: ["Add a regression test set of anonymised contracts."], src_marks: [], dst_marks: [[]] } }
    ]
  }]
};
// Remove the empty hunk headers used above as placeholders so indices stay simple.
window.MOCK.files[0].segments = window.MOCK.files[0].segments.filter(s => s.HunkHeader !== "");
window.MOCK.files[0].change_idx = window.MOCK.files[0].segments
  .map((s, i) => (s.Diff && s.Diff.kind !== "Unchanged") ? i : -1).filter(i => i >= 0);
