use std::io::Write;

use anyhow::{Context, Result};
use colored::{ColoredString, Colorize};
use dialoguer::Select;
use dialoguer::console::{Style, Term, measure_text_width, strip_ansi_codes, style};
use dialoguer::theme::ColorfulTheme;
use tempfile::NamedTempFile;

use crate::types::ActionType;

const MAX_PREVIEW_WIDTH: usize = 80;
const MIN_BOX_WIDTH: usize = 28;
const MIN_COMFORTABLE_BOX_WIDTH: usize = 48;
const BOX_HORIZONTAL_CHROME: usize = 4;

pub fn show_commit_options(message: &str) -> Result<ActionType> {
    let terminal_width = terminal_width();
    print_commit_preview(message, terminal_width);
    let navigation_hint = if terminal_width < MIN_COMFORTABLE_BOX_WIDTH {
        "↑↓ move  ·  enter select"
    } else {
        "↑↓ navigate  ·  enter select  ·  esc cancel"
    };
    println!("  {}", navigation_hint.dimmed());

    let items = [
        "Commit this message",
        "Edit in your editor",
        "Try another suggestion",
        "Cancel",
    ];
    let theme = prompt_theme();

    let selection = Select::with_theme(&theme)
        .with_prompt("Next step")
        .items(&items)
        .default(0)
        .report(false)
        .interact_opt()?;

    Ok(match selection {
        Some(0) => ActionType::Accept,
        Some(1) => ActionType::Edit,
        Some(2) => ActionType::Regenerate,
        _ => ActionType::Quit,
    })
}

pub fn prompt_theme() -> ColorfulTheme {
    ColorfulTheme {
        defaults_style: Style::new().for_stderr().cyan(),
        prompt_style: Style::new().for_stderr().bold(),
        prompt_prefix: style("◆".to_string()).for_stderr().cyan(),
        prompt_suffix: style("›".to_string()).for_stderr().black().bright(),
        success_prefix: style("✔".to_string()).for_stderr().green(),
        success_suffix: style("·".to_string()).for_stderr().black().bright(),
        error_prefix: style("✖".to_string()).for_stderr().red(),
        error_style: Style::new().for_stderr().red(),
        hint_style: Style::new().for_stderr().black().bright(),
        values_style: Style::new().for_stderr().cyan(),
        active_item_style: Style::new().for_stderr().bold(),
        inactive_item_style: Style::new().for_stderr().black().bright(),
        active_item_prefix: style("❯".to_string()).for_stderr().cyan().bold(),
        inactive_item_prefix: style(" ".to_string()).for_stderr(),
        ..ColorfulTheme::default()
    }
}

fn print_commit_preview(message: &str, terminal_width: usize) {
    let available_width = terminal_width.saturating_sub(2).min(MAX_PREVIEW_WIDTH);
    let sanitized = sanitize_terminal_text(message);

    if available_width < MIN_BOX_WIDTH {
        println!("\n{}", "Commit preview".bold().cyan());
        let content_width = available_width.saturating_sub(2).max(1);
        for (index, line) in wrap_preview(&sanitized, content_width).iter().enumerate() {
            println!("  {}", render_preview_line(line, index == 0));
        }
        println!();
        return;
    }

    let max_content_width = available_width - BOX_HORIZONTAL_CHROME;
    let preview_lines = wrap_preview(&sanitized, max_content_width);
    let widest_line = preview_lines
        .iter()
        .map(|line| measure_text_width(line))
        .max()
        .unwrap_or_default();
    let comfortable_width = MIN_COMFORTABLE_BOX_WIDTH.min(available_width);
    let box_width = (widest_line + BOX_HORIZONTAL_CHROME)
        .max(comfortable_width)
        .min(available_width);
    let content_width = box_width - BOX_HORIZONTAL_CHROME;
    let title = "Commit preview";
    let top_rule_width = box_width.saturating_sub(measure_text_width(title) + 5);

    println!();
    println!(
        "{} {} {}{}",
        "╭─".cyan(),
        title.bold().cyan(),
        "─".repeat(top_rule_width).cyan(),
        "╮".cyan(),
    );
    for (index, line) in preview_lines.iter().enumerate() {
        let padding = content_width.saturating_sub(measure_text_width(line));
        let content = render_preview_line(line, index == 0);
        println!(
            "{} {}{} {}",
            "│".cyan(),
            content,
            " ".repeat(padding),
            "│".cyan()
        );
    }
    println!("{}", format!("╰{}╯", "─".repeat(box_width - 2)).cyan());
    println!();
}

fn render_preview_line(line: &str, is_summary: bool) -> String {
    if is_summary {
        render_summary_line(line)
    } else if let Some(bullet) = line.strip_prefix("- ") {
        format!("{}{bullet}", "- ".bright_black())
    } else {
        line.to_string()
    }
}

fn render_summary_line(summary: &str) -> String {
    let Some((prefix, description)) = summary.split_once(": ") else {
        return summary.bold().to_string();
    };
    let (commit_type, scope) = match prefix.split_once('(') {
        Some((commit_type, scoped)) if scoped.ends_with(')') => {
            (commit_type, Some(&scoped[..scoped.len() - 1]))
        }
        _ => (prefix, None),
    };

    let mut rendered = String::new();
    let accent = accent_commit_type(commit_type);
    rendered.push_str(&accent.bold().to_string());
    if let Some(scope) = scope {
        rendered.push_str(&format!("({scope})").bright_black());
    }
    rendered.push_str(&": ".bright_black());
    rendered.push_str(description);
    rendered
}

fn accent_commit_type(commit_type: &str) -> ColoredString {
    match commit_type {
        "feat" => commit_type.bright_green(),
        "fix" => commit_type.bright_red(),
        "docs" => commit_type.blue(),
        "style" => commit_type.magenta(),
        "refactor" => commit_type.purple(),
        "perf" => commit_type.yellow(),
        "test" => commit_type.cyan(),
        "build" => commit_type.bright_blue(),
        "ci" => commit_type.bright_cyan(),
        "chore" => commit_type.bright_black(),
        "revert" => commit_type.bright_red(),
        _ => commit_type.normal(),
    }
}

fn terminal_width() -> usize {
    let columns = usize::from(Term::stdout().size().1);
    if columns == 0 {
        MAX_PREVIEW_WIDTH
    } else {
        columns
    }
}

pub(super) fn sanitize_terminal_text(message: &str) -> String {
    let stripped = strip_ansi_codes(message);
    let mut sanitized = String::with_capacity(stripped.len());
    for character in stripped.chars() {
        match character {
            '\n' => sanitized.push('\n'),
            '\t' => sanitized.push_str("    "),
            _ if character.is_control() || is_bidirectional_control(character) => {}
            _ => sanitized.push(character),
        }
    }
    sanitized
}

fn is_bidirectional_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn wrap_preview(message: &str, width: usize) -> Vec<String> {
    let mut wrapped = Vec::new();
    for line in message.lines() {
        wrapped.extend(wrap_line(line, width));
    }
    if wrapped.is_empty() {
        wrapped.push(String::new());
    }
    wrapped
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if line.trim().is_empty() {
        return vec![String::new()];
    }

    let trimmed = line.trim();
    let (first_prefix, continuation_prefix, content) =
        if let Some(content) = trimmed.strip_prefix("- ") {
            ("- ", "  ", content)
        } else {
            ("", "", trimmed)
        };
    let mut lines = Vec::new();
    let mut current = first_prefix.to_string();
    let mut active_prefix = first_prefix;

    for word in content.split_whitespace() {
        let separator = if current == active_prefix { "" } else { " " };
        let candidate = format!("{current}{separator}{word}");
        if measure_text_width(&candidate) <= width {
            current = candidate;
            continue;
        }

        if current != active_prefix {
            lines.push(current);
            active_prefix = continuation_prefix;
            current = active_prefix.to_string();
        }
        append_long_word(&mut lines, &mut current, active_prefix, word, width);
    }

    if current != active_prefix || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn append_long_word(
    lines: &mut Vec<String>,
    current: &mut String,
    prefix: &str,
    word: &str,
    width: usize,
) {
    for character in word.chars() {
        let mut encoded_character = [0; 4];
        let character = character.encode_utf8(&mut encoded_character);
        if measure_text_width(current) + measure_text_width(character) > width && current != prefix
        {
            lines.push(std::mem::replace(current, prefix.to_string()));
        }
        current.push_str(character);
    }
}

pub fn open_editor(content: &str) -> Result<String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let mut tmp_file = NamedTempFile::new().context("Failed to create temp file for editor")?;
    write!(tmp_file, "{}", content).context("Failed to write to temp file")?;
    let path = tmp_file.path().to_path_buf();

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to start editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with code: {:?}", status.code());
    }

    let edited_content =
        std::fs::read_to_string(&path).context("Failed to read edited content from temp file")?;

    Ok(edited_content.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_wraps_long_lines_to_the_available_width() {
        let lines = wrap_preview(
            "perf(cli): improve an unusually long terminal interface preview\n\n- preserve readable wrapping for every generated message",
            28,
        );

        assert!(lines.iter().all(|line| measure_text_width(line) <= 28));
        assert!(lines.iter().any(|line| line.starts_with("- ")));
        assert!(lines.iter().any(|line| line.starts_with("  for")));
    }

    #[test]
    fn preview_removes_terminal_and_bidirectional_controls() {
        let sanitized = sanitize_terminal_text("\x1b[31mfix(cli): safe preview\x1b[0m\x07\u{202e}");

        assert_eq!(sanitized, "fix(cli): safe preview");
    }

    #[test]
    fn summary_and_bullets_are_styled_without_changing_visible_text() {
        colored::control::set_override(true);
        let summary = render_preview_line("feat(auth): implement login", true);
        let bullet = render_preview_line("- add cache layer", false);
        colored::control::unset_override();

        assert_eq!(
            measure_text_width(&summary),
            measure_text_width("feat(auth): implement login")
        );
        assert_eq!(
            measure_text_width(&bullet),
            measure_text_width("- add cache layer")
        );
        assert!(summary.contains("feat"));
    }

    #[test]
    fn non_conventional_summary_renders_unchanged_without_color() {
        colored::control::set_override(false);
        let rendered = render_summary_line("just some text");
        colored::control::unset_override();

        assert_eq!(rendered, "just some text");
    }
}
