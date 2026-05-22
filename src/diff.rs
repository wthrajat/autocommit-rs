use crate::types::CommitType;

pub fn clean_diff(diff: &str) -> String {
    diff.lines()
        .filter(|line| {
            !line.starts_with("diff --git")
                && !line.starts_with("index ")
                && !line.starts_with("--- ")
                && !line.starts_with("+++ ")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn generate_prompt(
    diff: &str,
    commit_type: Option<CommitType>,
    files: &[String],
    branch_name: &str,
) -> String {
    let type_constraint = match commit_type {
        Some(t) => format!("Use type: {}.", t.as_str()),
        None => "Determine the best conventional commit type.".to_string(),
    };

    let files_info = if !files.is_empty() {
        format!("\n\nChanged files: {}", files.join(", "))
    } else {
        String::new()
    };

    let mut ticket_instruction = String::new();
    if !branch_name.is_empty()
        && let Some(ticket) = extract_ticket_id(branch_name)
    {
        ticket_instruction = format!(
            "\n\nIMPORTANT: The branch name contains ticket ID {}. You MUST append [{}] to the end of the commit summary line.",
            ticket, ticket
        );
    }

    format!("{type_constraint}{files_info}{ticket_instruction}\n\nGit diff:\n{diff}")
}

fn extract_ticket_id(branch_name: &str) -> Option<String> {
    // Match patterns like ABC-123, PROJ-42, etc.
    let chars: Vec<char> = branch_name.chars().collect();
    for i in 0..chars.len() {
        if chars[i].is_ascii_uppercase() {
            // Find the end of the uppercase sequence
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_uppercase() {
                j += 1;
            }
            // Check for dash followed by digits
            if j < chars.len() && chars[j] == '-' {
                let start = j + 1;
                if start < chars.len() && chars[start].is_ascii_digit() {
                    let mut k = start;
                    while k < chars.len() && chars[k].is_ascii_digit() {
                        k += 1;
                    }
                    let ticket: String = chars[i..k].iter().collect();
                    return Some(ticket);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_diff_removes_metadata_lines() {
        let diff = "diff --git a/file.ts b/file.ts\nindex abc..def 100644\n--- a/file.ts\n+++ b/file.ts\n@@ -1,3 +1,4 @@\n hello\n+world";
        let cleaned = clean_diff(diff);
        assert!(!cleaned.contains("diff --git"));
        assert!(!cleaned.contains("index "));
        assert!(!cleaned.contains("--- "));
        assert!(!cleaned.contains("+++ "));
        assert!(cleaned.contains("+world"));
    }

    #[test]
    fn test_generate_prompt_with_type() {
        let prompt = generate_prompt("test diff", Some(CommitType::Feat), &[], "");
        assert!(prompt.contains("Use type: feat."));
        assert!(prompt.contains("Git diff:\ntest diff"));
    }

    #[test]
    fn test_generate_prompt_with_files() {
        let prompt = generate_prompt("diff", None, &["src/main.rs".to_string()], "");
        assert!(prompt.contains("Changed files: src/main.rs"));
    }

    #[test]
    fn test_generate_prompt_with_ticket() {
        let prompt = generate_prompt("diff", None, &[], "feature/PROJ-123-add-auth");
        assert!(prompt.contains("[PROJ-123]"));
    }
}
