use std::io::Write;

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::Select;
use tempfile::NamedTempFile;

use crate::types::ActionType;

pub fn show_commit_options(message: &str) -> Result<ActionType> {
    println!("\n{}", "Generated commit message:".bold());
    println!("{}", "--------------------------------------------------".cyan());
    println!("{}", message);
    println!("{}", "--------------------------------------------------".cyan());
    println!();

    let items = vec![
        "Accept and commit",
        "Edit message",
        "Regenerate",
        "Quit",
    ];

    let selection = Select::new()
        .with_prompt("What would you like to do?")
        .items(&items)
        .default(0)
        .interact_opt()?;

    Ok(match selection {
        Some(0) => ActionType::Accept,
        Some(1) => ActionType::Edit,
        Some(2) => ActionType::Regenerate,
        _ => ActionType::Quit,
    })
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
