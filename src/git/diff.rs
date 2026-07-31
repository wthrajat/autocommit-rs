use std::io::BufRead;

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::types::CommitType;

const MAX_PATCH_BYTES: usize = 10_000;
const MAX_SOURCE_LINE_BYTES: usize = 2_048;
const MAX_FILE_LIST_BYTES: usize = 2_000;
const MAX_EXCLUDED_CANDIDATES_BYTES: usize = 2_000;

#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub raw_bytes: usize,
    pub included_bytes: usize,
    pub changed_files: usize,
    pub omitted_generated_files: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct DiffContext {
    pub patch: String,
    pub fingerprint: String,
    pub stats: DiffStats,
}

pub fn summarize_diff<R: BufRead>(mut reader: R, files: &[String]) -> Result<DiffContext> {
    let mut patch = String::with_capacity(MAX_PATCH_BYTES);
    let mut line_buffer = Vec::with_capacity(MAX_SOURCE_LINE_BYTES);
    let mut fingerprint = Sha256::new();
    let mut raw_bytes = 0;
    let mut current_file_bytes = 0;
    let mut omitted_generated_files = 0;
    let mut truncated = false;
    let mut skip_current_file = false;
    let mut remaining_meaningful_files = files
        .iter()
        .filter(|file| !is_generated_path(file))
        .count()
        .max(1);
    let mut current_file_budget = MAX_PATCH_BYTES;

    while let Some(line_was_truncated) = read_bounded_line(
        &mut reader,
        &mut line_buffer,
        &mut fingerprint,
        &mut raw_bytes,
    )? {
        truncated |= line_was_truncated;
        let line = String::from_utf8_lossy(&line_buffer);
        let line = line.trim_end_matches(['\r', '\n']);

        if line.starts_with("diff --git ") {
            current_file_bytes = 0;
            skip_current_file = is_generated_diff_header(line);
            if skip_current_file {
                omitted_generated_files += 1;
                current_file_budget = 256;
            } else {
                current_file_budget =
                    MAX_PATCH_BYTES.saturating_sub(patch.len()) / remaining_meaningful_files;
                remaining_meaningful_files = remaining_meaningful_files.saturating_sub(1).max(1);
            }

            let available = current_file_budget.saturating_sub(current_file_bytes);
            truncated |= !line_fits(&patch, line, available);
            let added = append_line(&mut patch, line, available);
            current_file_bytes += added;

            if skip_current_file {
                let added = append_line(
                    &mut patch,
                    "[patch omitted: generated, vendored, or lock file]",
                    current_file_budget.saturating_sub(current_file_bytes),
                );
                current_file_bytes += added;
            }
            continue;
        }

        if skip_current_file || is_redundant_metadata(line) {
            continue;
        }

        let available_for_file = current_file_budget.saturating_sub(current_file_bytes);
        let line_fits = line_fits(&patch, line, available_for_file);
        let added = append_line(&mut patch, line, available_for_file);
        current_file_bytes += added;
        if !line_fits {
            truncated = true;
        }
    }

    let patch = patch.trim().to_string();
    let patch = if patch.is_empty() {
        "No textual patch was produced; infer the change from the staged file list.".to_string()
    } else {
        patch
    };
    let fingerprint = format!("{:x}", fingerprint.finalize());
    let included_bytes = patch.len();

    Ok(DiffContext {
        patch,
        fingerprint,
        stats: DiffStats {
            raw_bytes,
            included_bytes,
            changed_files: files.len(),
            omitted_generated_files,
            truncated,
        },
    })
}

pub fn generate_prompt(
    diff: &str,
    commit_type: Option<CommitType>,
    files: &[String],
    branch_name: &str,
    excluded_candidates: &[String],
) -> String {
    let type_hint = match commit_type {
        Some(commit_type) => format!(
            "Likely commit type: {}. Treat this as a hint and override it if the patch indicates a better type.",
            commit_type.as_str()
        ),
        None => "Determine the best conventional commit type from the patch.".to_string(),
    };
    let changed_files = format_changed_files(files);
    let ticket_instruction = extract_ticket_id(branch_name).map_or_else(String::new, |ticket| {
        format!("\nTicket: append [{ticket}] to the end of every candidate's summary line.")
    });
    let exclusions = format_excluded_candidates(excluded_candidates);

    format!(
        "{type_hint}\nChanged files: {changed_files}{ticket_instruction}\n\nGit diff:\n{diff}{exclusions}"
    )
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    line: &mut Vec<u8>,
    fingerprint: &mut Sha256,
    raw_bytes: &mut usize,
) -> std::io::Result<Option<bool>> {
    line.clear();
    let mut line_was_truncated = false;
    let mut read_anything = false;

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(read_anything.then_some(line_was_truncated));
        }

        read_anything = true;
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        fingerprint.update(&available[..consumed]);
        *raw_bytes += consumed;

        let remaining = MAX_SOURCE_LINE_BYTES.saturating_sub(line.len());
        let copied = remaining.min(consumed);
        line.extend_from_slice(&available[..copied]);
        line_was_truncated |= copied < consumed;
        let found_newline = available[..consumed].ends_with(b"\n");
        reader.consume(consumed);

        if found_newline {
            return Ok(Some(line_was_truncated));
        }
    }
}

fn append_line(output: &mut String, line: &str, file_budget: usize) -> usize {
    let global_budget = MAX_PATCH_BYTES.saturating_sub(output.len());
    let available = global_budget.min(file_budget);
    let separator_bytes = usize::from(!output.is_empty());
    if available <= separator_bytes {
        return 0;
    }

    let content_budget = available - separator_bytes;
    let content = utf8_prefix(line, content_budget);
    if content.is_empty() && !line.is_empty() {
        return 0;
    }

    if separator_bytes == 1 {
        output.push('\n');
    }
    output.push_str(content);
    separator_bytes + content.len()
}

fn line_fits(output: &str, line: &str, file_budget: usize) -> bool {
    let global_budget = MAX_PATCH_BYTES.saturating_sub(output.len());
    let available = global_budget.min(file_budget);
    let separator_bytes = usize::from(!output.is_empty());
    separator_bytes + line.len() <= available
}

fn utf8_prefix(value: &str, max_bytes: usize) -> &str {
    let mut end = value.len().min(max_bytes);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

fn is_generated_diff_header(line: &str) -> bool {
    is_generated_path(line)
}

fn is_generated_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        "cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock",
        ".min.js",
        ".min.css",
        "/vendor/",
        "/dist/",
        "/generated/",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_redundant_metadata(line: &str) -> bool {
    line.starts_with("index ") || line.starts_with("--- ") || line.starts_with("+++ ")
}

fn format_changed_files(files: &[String]) -> String {
    if files.is_empty() {
        return "none".to_string();
    }

    let mut formatted = String::new();
    let mut included = 0;
    for file in files {
        let separator = if formatted.is_empty() { "" } else { ", " };
        if formatted.len() + separator.len() + file.len() > MAX_FILE_LIST_BYTES {
            break;
        }
        formatted.push_str(separator);
        formatted.push_str(file);
        included += 1;
    }

    let omitted = files.len() - included;
    if omitted > 0 {
        formatted.push_str(&format!(", ... (+{omitted} more)"));
    }
    formatted
}

fn format_excluded_candidates(candidates: &[String]) -> String {
    if candidates.is_empty() {
        return String::new();
    }

    let mut formatted = String::from(
        "\n\nGenerate new alternatives that are meaningfully different from these previous candidates:",
    );
    for candidate in candidates.iter().rev().take(6).rev() {
        let candidate = candidate.replace('\n', " ");
        let available = MAX_EXCLUDED_CANDIDATES_BYTES.saturating_sub(formatted.len());
        if available <= 3 {
            break;
        }
        formatted.push_str("\n- ");
        formatted.push_str(utf8_prefix(&candidate, available - 3));
    }
    formatted
}

fn extract_ticket_id(branch_name: &str) -> Option<&str> {
    let bytes = branch_name.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if !bytes[index].is_ascii_uppercase() {
            index += 1;
            continue;
        }

        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_uppercase() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'-' {
            continue;
        }
        index += 1;
        let digits_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index > digits_start {
            return branch_name.get(start..index);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn summarize_diff_removes_metadata_and_balances_files() {
        let diff = "diff --git a/one.rs b/one.rs\nindex abc..def 100644\n--- a/one.rs\n+++ b/one.rs\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/two.rs b/two.rs\n@@ -1 +1 @@\n-before\n+after\n";
        let files = ["one.rs".to_string(), "two.rs".to_string()];
        let context = summarize_diff(Cursor::new(diff), &files).unwrap();

        assert!(!context.patch.contains("index "));
        assert!(!context.patch.contains("--- "));
        assert!(context.patch.contains("a/one.rs"));
        assert!(context.patch.contains("a/two.rs"));
        assert_eq!(context.stats.changed_files, 2);
    }

    #[test]
    fn summarize_diff_omits_generated_file_patches() {
        let diff = "diff --git a/Cargo.lock b/Cargo.lock\n@@ -1 +1 @@\n-old\n+new\n";
        let context = summarize_diff(Cursor::new(diff), &["Cargo.lock".to_string()]).unwrap();

        assert!(context.patch.contains("patch omitted"));
        assert!(!context.patch.contains("+new"));
        assert_eq!(context.stats.omitted_generated_files, 1);
    }

    #[test]
    fn summarize_diff_reallocates_unused_budget_to_later_files() {
        let large_change = "+x\n".repeat(2_500);
        let diff = format!(
            "diff --git a/small.rs b/small.rs\n+small\ndiff --git a/large.rs b/large.rs\n{large_change}"
        );
        let files = ["small.rs".to_string(), "large.rs".to_string()];
        let context = summarize_diff(Cursor::new(diff), &files).unwrap();

        assert!(context.patch.len() > MAX_PATCH_BYTES / 2);
        assert!(context.patch.len() <= MAX_PATCH_BYTES);
    }

    #[test]
    fn summarize_diff_bounds_long_lines_and_hashes_discarded_tail() {
        let prefix = "+".repeat(MAX_SOURCE_LINE_BYTES + 500);
        let first = format!("diff --git a/a.rs b/a.rs\n{prefix}x\n");
        let second = format!("diff --git a/a.rs b/a.rs\n{prefix}y\n");
        let files = ["a.rs".to_string()];
        let first_context = summarize_diff(Cursor::new(&first), &files).unwrap();
        let second_context = summarize_diff(Cursor::new(&second), &files).unwrap();

        assert!(first_context.patch.len() <= MAX_PATCH_BYTES);
        assert!(first_context.stats.truncated);
        assert_ne!(first_context.fingerprint, second_context.fingerprint);
    }

    #[test]
    fn generate_prompt_uses_type_as_hint_and_limits_file_list() {
        let files = (0..500)
            .map(|index| format!("src/very-long-file-name-{index}.rs"))
            .collect::<Vec<_>>();
        let prompt = generate_prompt(
            "test diff",
            Some(CommitType::Feat),
            &files,
            "feature/PROJ-123-add-auth",
            &[],
        );

        assert!(prompt.contains("Likely commit type: feat"));
        assert!(prompt.contains("[PROJ-123]"));
        assert!(prompt.contains("more)"));
        assert!(prompt.contains("Git diff:\ntest diff"));
    }

    #[test]
    fn generate_prompt_lists_previous_candidates_last() {
        let prompt = generate_prompt(
            "diff",
            None,
            &[],
            "",
            &["feat(core): first option".to_string()],
        );

        assert!(prompt.ends_with("- feat(core): first option"));
    }
}
