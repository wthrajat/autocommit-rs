use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn is_git_repository() -> Result<()> {
    let status = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .context("Failed to check if inside git repository")?;
    if !status.status.success() {
        bail!("Not a git repository");
    }
    Ok(())
}

pub fn has_staged_changes() -> Result<bool> {
    let status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()
        .context("Failed to check for staged changes")?;
    // Exit code 0 = no differences, 1 = has differences
    Ok(!status.success())
}

pub fn get_staged_diff() -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--no-color", "--ignore-all-space"])
        .output()
        .context("Failed to get staged diff")?;
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

pub fn get_changed_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .output()
        .context("Failed to get changed files")?;
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.lines().map(|s| s.to_string()).filter(|s| !s.is_empty()).collect())
}

pub fn commit_changes(message: &str, signed: bool, no_verify: bool) -> Result<()> {
    let mut args = vec!["commit"];
    if signed {
        args.push("-S");
    }
    if no_verify {
        args.push("--no-verify");
    }
    args.push("-m");
    args.push(message);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to commit changes")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Git commit failed: {}", stderr.trim());
    }
    Ok(())
}

pub fn get_branch_name() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to get branch name")?;
    if !output.status.success() {
        return Ok(String::new());
    }
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.trim().to_string())
}
