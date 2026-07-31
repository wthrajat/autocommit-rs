use std::path::Path;

use crate::types::CommitType;

pub fn classify_files(files: &[String]) -> Option<CommitType> {
    if files.is_empty() {
        return None;
    }

    if files.iter().all(|file| is_test_file(file)) {
        return Some(CommitType::Test);
    }
    if files.iter().all(|file| is_documentation_file(file)) {
        return Some(CommitType::Docs);
    }
    if files.iter().all(|file| is_configuration_file(file)) {
        return Some(CommitType::Chore);
    }
    None
}

fn is_test_file(file: &str) -> bool {
    let lower = file.to_ascii_lowercase();
    let path = Path::new(&lower);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some("test" | "tests" | "spec" | "specs" | "__tests__")
        )
    }) || file_name.contains("_test.")
        || file_name.contains(".test.")
        || file_name.contains("_spec.")
        || file_name.contains(".spec.")
}

fn is_documentation_file(file: &str) -> bool {
    let lower = file.to_ascii_lowercase();
    let path = Path::new(&lower);
    let extension = path.extension().and_then(|extension| extension.to_str());

    matches!(extension, Some("md" | "mdx" | "rst" | "adoc"))
        || path
            .components()
            .any(|component| component.as_os_str() == "docs")
}

fn is_configuration_file(file: &str) -> bool {
    let lower = file.to_ascii_lowercase();
    let file_name = Path::new(&lower)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    matches!(
        file_name,
        "cargo.toml"
            | "cargo.lock"
            | "package.json"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "bun.lock"
            | "tsconfig.json"
            | "rust-toolchain.toml"
    ) || file_name.starts_with(".env")
        || file_name.starts_with(".eslint")
        || file_name.starts_with(".prettier")
        || file_name.ends_with(".config.js")
        || file_name.ends_with(".config.ts")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_test_files_without_substring_false_positives() {
        assert_eq!(
            classify_files(&["tests/foo.rs".to_string(), "src/bar_test.rs".to_string()]),
            Some(CommitType::Test)
        );
        assert_eq!(classify_files(&["src/latest.rs".to_string()]), None);
    }

    #[test]
    fn classifies_documentation_files() {
        assert_eq!(
            classify_files(&["README.md".to_string(), "docs/guide.txt".to_string()]),
            Some(CommitType::Docs)
        );
    }

    #[test]
    fn classifies_configuration_files() {
        assert_eq!(
            classify_files(&["Cargo.toml".to_string(), "Cargo.lock".to_string()]),
            Some(CommitType::Chore)
        );
    }

    #[test]
    fn leaves_mixed_or_source_changes_for_the_model() {
        assert_eq!(
            classify_files(&["README.md".to_string(), "src/main.rs".to_string()]),
            None
        );
    }
}
