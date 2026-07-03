//! Unified-diff parser — port of `DiffParser` in `DiffViewModel.swift`.
//! Hand-rolled (no regex crate) to keep the dependency tree minimal.

use super::models::{DiffFile, DiffHunk, DiffLine, DiffLineType, FileChangeType};

/// Extract old/new start line numbers from an `@@ -a,b +c,d @@` header.
fn parse_hunk_starts(line: &str) -> (usize, usize) {
    let mut old_start = 0usize;
    let mut new_start = 0usize;
    for tok in line.split_whitespace() {
        if let Some(rest) = tok.strip_prefix('-') {
            old_start = rest.split(',').next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(rest) = tok.strip_prefix('+') {
            new_start = rest.split(',').next().and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
    (old_start, new_start)
}

pub fn parse(raw: &str, file_path: &str) -> DiffFile {
    if raw.is_empty() {
        return DiffFile {
            path: file_path.to_string(),
            hunks: Vec::new(),
            is_binary: false,
            file_status: FileChangeType::Modified,
        };
    }
    if raw.contains("Binary files") {
        return DiffFile {
            path: file_path.to_string(),
            hunks: Vec::new(),
            is_binary: true,
            file_status: FileChangeType::Modified,
        };
    }

    let file_status = if raw.contains("new file mode") {
        FileChangeType::Added
    } else if raw.contains("deleted file mode") {
        FileChangeType::Deleted
    } else if raw.contains("rename from") {
        FileChangeType::Renamed
    } else {
        FileChangeType::Modified
    };

    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut cur_lines: Vec<DiffLine> = Vec::new();
    let mut cur_header = String::new();
    let mut old_line = 0usize;
    let mut new_line = 0usize;
    let mut in_hunk = false;

    for line in raw.split('\n') {
        if line.starts_with("@@") {
            if in_hunk && !cur_lines.is_empty() {
                hunks.push(DiffHunk { header: std::mem::take(&mut cur_header), lines: std::mem::take(&mut cur_lines) });
            } else {
                cur_lines.clear();
            }
            // trim trailing function context after the closing " @@"
            cur_header = match line.find(" @@") {
                Some(pos) => line[..pos + 3].to_string(),
                None => line.to_string(),
            };
            let (o, n) = parse_hunk_starts(line);
            old_line = o;
            new_line = n;
            in_hunk = true;
        } else if in_hunk {
            if let Some(rest) = line.strip_prefix('+') {
                cur_lines.push(DiffLine {
                    line_type: DiffLineType::Added,
                    content: rest.to_string(),
                    old_line_number: None,
                    new_line_number: Some(new_line),
                });
                new_line += 1;
            } else if let Some(rest) = line.strip_prefix('-') {
                cur_lines.push(DiffLine {
                    line_type: DiffLineType::Removed,
                    content: rest.to_string(),
                    old_line_number: Some(old_line),
                    new_line_number: None,
                });
                old_line += 1;
            } else if let Some(rest) = line.strip_prefix(' ') {
                cur_lines.push(DiffLine {
                    line_type: DiffLineType::Context,
                    content: rest.to_string(),
                    old_line_number: Some(old_line),
                    new_line_number: Some(new_line),
                });
                old_line += 1;
                new_line += 1;
            }
            // "\ No newline at end of file" and headers before the first hunk are ignored
        }
    }
    if in_hunk && !cur_lines.is_empty() {
        hunks.push(DiffHunk { header: cur_header, lines: cur_lines });
    }

    DiffFile { path: file_path.to_string(), hunks, is_binary: false, file_status }
}

/// Build an all-added diff for an untracked file (git diff is empty for these).
pub fn synthesize_added(content: &str, file_path: &str) -> DiffFile {
    let mut lines: Vec<&str> = content.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let diff_lines: Vec<DiffLine> = lines
        .iter()
        .enumerate()
        .map(|(idx, text)| DiffLine {
            line_type: DiffLineType::Added,
            content: (*text).to_string(),
            old_line_number: None,
            new_line_number: Some(idx + 1),
        })
        .collect();

    let hunks = if lines.is_empty() {
        Vec::new()
    } else {
        vec![DiffHunk {
            header: format!("@@ -0,0 +1,{} @@", lines.len()),
            lines: diff_lines,
        }]
    };

    DiffFile { path: file_path.to_string(), hunks, is_binary: false, file_status: FileChangeType::Added }
}
