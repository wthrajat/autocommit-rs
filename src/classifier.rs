use crate::types::CommitType;

pub fn classify_diff(files: &[String], diff: &str) -> Option<CommitType> {
    if files.is_empty() {
        return classify_by_diff_content(diff);
    }

    let is_only_tests = files
        .iter()
        .all(|f| f.contains("test") || f.contains("spec"));
    if is_only_tests {
        return Some(CommitType::Test);
    }

    let is_only_docs = files
        .iter()
        .all(|f| f.ends_with(".md") || f.contains("docs/"));
    if is_only_docs {
        return Some(CommitType::Docs);
    }

    let is_only_config = files.iter().all(|f| {
        f.contains("package.json")
            || f.contains(".env")
            || f.contains("tsconfig")
            || f.contains(".eslintrc")
            || f.contains(".prettierrc")
    });
    if is_only_config {
        return Some(CommitType::Chore);
    }

    classify_by_diff_content(diff)
}

fn classify_by_diff_content(diff: &str) -> Option<CommitType> {
    let lower_diff = diff.to_lowercase();

    if lower_diff.contains("fix") || lower_diff.contains("bug") || lower_diff.contains("error") {
        return Some(CommitType::Fix);
    }

    // If many lines are deleted and added, it might be a refactor
    let deleted_lines = diff.lines().filter(|l| l.starts_with('-')).count();
    let added_lines = diff.lines().filter(|l| l.starts_with('+')).count();
    if deleted_lines > 50 && added_lines > 50 {
        return Some(CommitType::Refactor);
    }

    // If new files are created
    if diff.contains("new file mode") {
        return Some(CommitType::Feat);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_diff_test_files() {
        let files = vec!["src/test.rs".to_string(), "tests/foo.rs".to_string()];
        assert_eq!(classify_diff(&files, ""), Some(CommitType::Test));
    }

    #[test]
    fn test_classify_diff_docs_files() {
        let files = vec!["README.md".to_string(), "docs/guide.md".to_string()];
        assert_eq!(classify_diff(&files, ""), Some(CommitType::Docs));
    }

    #[test]
    fn test_classify_diff_config_files() {
        let files = vec!["package.json".to_string(), ".env".to_string()];
        assert_eq!(classify_diff(&files, ""), Some(CommitType::Chore));
    }

    #[test]
    fn test_classify_diff_fix_by_content() {
        let files = vec!["src/main.rs".to_string()];
        let diff = "fix: correct the bug in the authentication flow";
        assert_eq!(classify_diff(&files, diff), Some(CommitType::Fix));
    }

    #[test]
    fn test_classify_diff_new_files() {
        let files = vec!["src/new_module.rs".to_string()];
        let diff = "new file mode";
        assert_eq!(classify_diff(&files, diff), Some(CommitType::Feat));
    }

    #[test]
    fn test_classify_diff_null_for_normal() {
        let files = vec!["src/utils.rs".to_string()];
        let diff = "some normal changes without keywords";
        assert_eq!(classify_diff(&files, diff), None);
    }
}
