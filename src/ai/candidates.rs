use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::types::MessageStyle;

use super::prompts::CANDIDATE_COUNT;

#[derive(Deserialize)]
struct CandidatePayload {
    candidates: Vec<String>,
}

pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "candidates": {
                "type": "array",
                "description": "Exactly three distinct complete commit messages ordered from strongest to weakest.",
                "items": {
                    "type": "string",
                    "description": "A complete commit message ready to pass to git commit."
                },
                "minItems": CANDIDATE_COUNT,
                "maxItems": CANDIDATE_COUNT
            }
        },
        "required": ["candidates"],
        "additionalProperties": false
    })
}

pub(crate) fn parse(
    content: &str,
    message_style: MessageStyle,
    excluded_candidates: &[String],
) -> Result<Vec<String>> {
    let payload = serde_json::from_str::<CandidatePayload>(content)
        .context("AI returned an invalid structured commit-message response")?;
    normalize(payload.candidates, message_style, excluded_candidates)
}

pub(crate) fn normalize(
    candidates: Vec<String>,
    message_style: MessageStyle,
    excluded_candidates: &[String],
) -> Result<Vec<String>> {
    let mut normalized = Vec::with_capacity(CANDIDATE_COUNT);
    for candidate in candidates {
        let candidate = candidate.replace("\r\n", "\n").trim().to_string();
        if candidate.is_empty()
            || excluded_candidates.contains(&candidate)
            || normalized.contains(&candidate)
            || !is_valid_commit_message(&candidate, message_style)
        {
            continue;
        }
        normalized.push(candidate);
    }

    if normalized.is_empty() {
        bail!("AI did not return a new valid Conventional Commit message");
    }
    Ok(normalized)
}

fn is_valid_commit_message(candidate: &str, message_style: MessageStyle) -> bool {
    if candidate.contains("```") || candidate.chars().any(is_unsafe_character) {
        return false;
    }

    let summary = candidate.lines().next().unwrap_or_default();
    if summary.chars().count() > 72 || summary.ends_with('.') {
        return false;
    }
    let Some(separator) = summary.find(": ") else {
        return false;
    };
    let prefix = &summary[..separator];
    let Some(scope_start) = prefix.find('(') else {
        return false;
    };
    if !prefix.ends_with(')') || scope_start + 2 > prefix.len() {
        return false;
    }

    let commit_type = &prefix[..scope_start];
    let valid_type = [
        "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
        "revert",
    ]
    .contains(&commit_type);
    let scope = &prefix[scope_start + 1..prefix.len() - 1];
    let description = &summary[separator + 2..];
    if !valid_type || scope.is_empty() || description.is_empty() {
        return false;
    }

    match message_style {
        MessageStyle::Short => !candidate.contains('\n'),
        MessageStyle::Long => {
            let body = &candidate[summary.len()..];
            body.starts_with("\n\n- ")
                && body[2..]
                    .lines()
                    .all(|line| line.starts_with("- ") && line.len() > 2)
        }
    }
}

fn is_unsafe_character(character: char) -> bool {
    (character.is_control() && character != '\n')
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_requires_exact_candidate_count() {
        let schema = schema();
        assert_eq!(
            schema["properties"]["candidates"]["minItems"],
            CANDIDATE_COUNT
        );
        assert_eq!(
            schema["properties"]["candidates"]["maxItems"],
            CANDIDATE_COUNT
        );
    }

    #[test]
    fn normalizes_valid_unique_candidates() {
        let candidates = vec![
            "feat(core): add cache".to_string(),
            "feat(core): add cache".to_string(),
            "perf(core): reuse cached messages".to_string(),
        ];
        let normalized = normalize(candidates, MessageStyle::Short, &[]).unwrap();

        assert_eq!(normalized.len(), 2);
    }

    #[test]
    fn rejects_invalid_or_previously_used_candidates() {
        let candidates = vec![
            "not conventional".to_string(),
            "fix(core): previous".to_string(),
        ];
        let error = normalize(
            candidates,
            MessageStyle::Short,
            &["fix(core): previous".to_string()],
        )
        .unwrap_err();

        assert!(error.to_string().contains("did not return"));
    }

    #[test]
    fn validates_long_message_bullet_format_without_network_access() {
        assert!(is_valid_commit_message(
            "perf(ai): reduce generation cost\n\n- batch alternatives\n- bound diff context",
            MessageStyle::Long
        ));
        assert!(!is_valid_commit_message(
            "perf(ai): reduce generation cost\nmissing blank line",
            MessageStyle::Long
        ));
        assert!(!is_valid_commit_message(
            "fix(cli): hide text\u{202e}",
            MessageStyle::Short
        ));
    }
}
