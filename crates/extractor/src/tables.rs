//! Table detection from plain-text layout.
//!
//! Detects tables by identifying runs of consecutive lines that share the same
//! column-divider structure (repeated runs of 2+ spaces, pipes, or tabs). A
//! header is inferred when the first row is followed by a separator line of
//! dashes or equals signs.

use crate::models::Table;

/// Detects tables in text. Stateless and cheap to construct.
pub struct TableDetector {
    /// Minimum number of aligned rows to qualify as a table.
    pub min_rows: usize,
}

impl Default for TableDetector {
    fn default() -> Self {
        TableDetector { min_rows: 2 }
    }
}

impl TableDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan `text` and return all detected tables.
    pub fn detect(&self, text: &str) -> Vec<Table> {
        let lines: Vec<&str> = text.lines().collect();
        let mut tables = Vec::new();

        let mut i = 0;
        while i < lines.len() {
            let candidate = self.detect_starting_at(&lines, i);
            if let Some(table) = candidate {
                let row_count = table.rows.len();
                tables.push(table);
                i += row_count.max(1);
            } else {
                i += 1;
            }
        }

        tables
    }

    fn detect_starting_at(&self, lines: &[&str], start: usize) -> Option<Table> {
        let first = lines.get(start)?;
        let cols = split_columns(first);
        if cols.len() < 2 {
            return None;
        }

        // Check if the second line is a separator (----, ====, |---|).
        let has_separator = lines
            .get(start + 1)
            .map(|l| is_separator_line(l))
            .unwrap_or(false);

        // Collect consecutive lines with the same column count.
        let mut rows = Vec::new();
        for (offset, line) in lines[start..].iter().enumerate() {
            if offset == 1 && has_separator {
                continue; // skip the separator line
            }
            let line_cols = split_columns(line);
            if line_cols.len() == cols.len() && !line.trim().is_empty() {
                rows.push(line_cols);
            } else {
                break;
            }
        }

        if rows.len() < self.min_rows {
            return None;
        }

        Some(Table {
            rows,
            has_header: has_separator || self.looks_like_header(&cols),
            start_line: start + 1,
        })
    }

    /// Heuristic: a row looks like a header if its cells are all short and
    /// contain no digits (e.g. ["Name", "Email", "Amount"]).
    fn looks_like_header(&self, cols: &[String]) -> bool {
        cols.iter().all(|c| {
            c.len() <= 20 && !c.chars().any(|c| c.is_ascii_digit())
        })
    }
}

/// Split a line into columns on: 2+ spaces, tabs, or pipe characters.
fn split_columns(line: &str) -> Vec<String> {
    // Pipe-delimited (markdown-style)
    if line.contains('|') && line.trim().starts_with('|') {
        return line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect();
    }

    // Tab-delimited
    if line.contains('\t') {
        return line.split('\t').map(|c| c.trim().to_string()).collect();
    }

    // 2+ space delimited
    let re = regex_static();
    let mut cols = Vec::new();
    let mut last_end = 0;
    for m in re.find_iter(line) {
        if m.start() > last_end {
            cols.push(line[last_end..m.start()].trim().to_string());
        }
        last_end = m.end();
    }
    if last_end < line.len() {
        let last = line[last_end..].trim();
        if !last.is_empty() {
            cols.push(last.to_string());
        }
    }
    cols
}

fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    // All dashes, equals, pipes, spaces, or colons (e.g. "|---|---|").
    trimmed.chars().all(|c| matches!(c, '-' | '=' | '|' | ' ' | ':'))
        && trimmed.chars().filter(|c| *c == '-' || *c == '=').count() >= 2
}

fn regex_static() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r" {2,}").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_space_aligned_table() {
        let text = "Name       Email              Amount\nAlice      alice@x.com        100\nBob        bob@y.com          200";
        let tables = TableDetector::new().detect(text);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].row_count(), 3);
        assert_eq!(tables[0].column_count(), 3);
        assert!(tables[0].has_header);
    }

    #[test]
    fn detects_pipe_table() {
        let text = "| Name  | Age |\n|-------|-----|\n| Alice | 30  |";
        let tables = TableDetector::new().detect(text);
        assert_eq!(tables.len(), 1);
        assert!(tables[0].has_header);
    }

    #[test]
    fn ignores_single_line() {
        let text = "Just one line of prose here.";
        let tables = TableDetector::new().detect(text);
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn detects_tab_delimited() {
        let text = "Name\tAge\tCity\nAlice\t30\tNYC\nBob\t25\tLA";
        let tables = TableDetector::new().detect(text);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].column_count(), 3);
    }
}
