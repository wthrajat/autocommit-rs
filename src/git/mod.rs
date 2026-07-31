use std::io::BufReader;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use self::diff::DiffContext;

pub mod diff;

#[derive(Debug, Clone)]
pub struct RepositoryState {
    pub branch_name: String,
    pub staged_files: Vec<String>,
}

pub fn get_repository_state() -> Result<RepositoryState> {
    let output = Command::new("git")
        .args([
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=no",
        ])
        .output()
        .context("Failed to inspect the Git repository")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Not a Git repository: {}", stderr.trim());
    }

    Ok(parse_repository_state(&output.stdout))
}

pub fn get_staged_diff_context(files: &[String]) -> Result<DiffContext> {
    let mut child = Command::new("git")
        .args([
            "diff",
            "--cached",
            "--no-color",
            "--no-ext-diff",
            "--unified=2",
            "--",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start the staged diff")?;
    let stdout = child
        .stdout
        .take()
        .context("Failed to capture the staged diff")?;

    let context = match diff::summarize_diff(BufReader::new(stdout), files) {
        Ok(context) => context,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error).context("Failed to summarize the staged diff");
        }
    };
    let output = child
        .wait_with_output()
        .context("Failed to finish the staged diff")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to get staged diff: {}", stderr.trim());
    }

    Ok(context)
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

fn parse_repository_state(output: &[u8]) -> RepositoryState {
    let mut branch_name = String::new();
    let mut staged_files = Vec::new();
    let mut skip_rename_source = false;

    for raw_record in output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        if skip_rename_source {
            skip_rename_source = false;
            continue;
        }

        let record = String::from_utf8_lossy(raw_record);
        if let Some(branch) = record.strip_prefix("# branch.head ") {
            if branch != "(detached)" {
                branch_name = branch.to_string();
            }
            continue;
        }

        let record_type = record.as_bytes().first().copied();
        let field_count = match record_type {
            Some(b'1') => 9,
            Some(b'2') => {
                skip_rename_source = true;
                10
            }
            Some(b'u') => 11,
            _ => continue,
        };
        let fields = record.splitn(field_count, ' ').collect::<Vec<_>>();
        if fields.len() != field_count {
            continue;
        }

        let status = fields[1].as_bytes().first().copied();
        if status != Some(b'.') {
            staged_files.push(fields[field_count - 1].to_string());
        }
    }

    RepositoryState {
        branch_name,
        staged_files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_and_only_staged_files() {
        let status = b"# branch.head feature/APP-42-cache\0\
1 M. N... 100644 100644 100644 abc def src/main.rs\0\
1 .M N... 100644 100644 100644 abc def README.md\0";
        let state = parse_repository_state(status);

        assert_eq!(state.branch_name, "feature/APP-42-cache");
        assert_eq!(state.staged_files, ["src/main.rs"]);
    }

    #[test]
    fn parses_renames_without_treating_source_as_a_record() {
        let status = b"# branch.head main\0\
2 R. N... 100644 100644 100644 abc def R100 src/new name.rs\0\
src/old name.rs\0\
1 A. N... 000000 100644 100644 000 def src/next.rs\0";
        let state = parse_repository_state(status);

        assert_eq!(state.staged_files, ["src/new name.rs", "src/next.rs"]);
    }

    #[test]
    fn hides_detached_head_marker() {
        let state = parse_repository_state(b"# branch.head (detached)\0");

        assert!(state.branch_name.is_empty());
    }
}
