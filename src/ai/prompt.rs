use std::collections::HashSet;

use crate::{
    ai::client::CommitContext,
    config::{CommitConfig, RuleLevel},
};

pub fn build_system_prompt() -> String {
    "you are an expert software engineer who writes excellent conventional commit messages. respond with only the commit message and body, no explanation or additional text.".to_string()
}

/// Builds the prompt for commit generation.
pub fn build_prompt(
    staged_changes: &str,
    context: &CommitContext,
    config: &CommitConfig,
) -> String {
    // introduction
    let intro_paragraph = "generate a conventional commit message for these staged changes:";

    // diff
    let diff_paragraph = format!(
        "```diff
{staged_changes}
```"
    );

    // branch
    let branch_paragraph = context
        .branch_name
        .as_ref()
        .map(|branch| format!("you are on a branch named `{branch}`."))
        .unwrap_or_default();

    // last 10 commits
    let recent_commits_paragraph = if !context.recent_commits.is_empty() {
        let mut points = vec!["recent commit messages for context:".into()];
        for commit in context.recent_commits.iter().take(10) {
            points.push(format!("- {}", commit));
        }

        points.join("\n")
    } else {
        Default::default()
    };

    // types
    let types_list = set_to_list_string(&config.types);

    let mut requirements = vec![
        "requirements:".into(),
        "- use conventional commits format: `type(scope): summary`".into(),
        format!("- type MUST be one of {types_list}"),
    ];

    // gather deny rules
    let RuleLevel {
        subject_length: deny_subject_length,
        body_length: deny_body_length,
        no_body: deny_no_body,
        no_scope: deny_no_scope,
        no_breaking_change_footer: deny_no_breaking_change_footer,
        ..
    } = config.rules.deny;

    // gather warn rules
    let RuleLevel {
        subject_length: warn_subject_length,
        body_length: warn_body_length,
        no_body: warn_no_body,
        no_scope: warn_no_scope,
        no_breaking_change_footer: warn_no_breaking_change_footer,
        ..
    } = config.rules.warn;

    // scope presence
    requirements.push(negative_rule_to_must_should(
        "summary",
        "scope",
        deny_no_scope,
        warn_no_scope,
    ));

    // scope set
    if let Some(scopes) = &config.scopes {
        let scopes_list = set_to_list_string(scopes);
        requirements.push(format!("- scope MUST be one of {scopes_list}"));
    }

    // summary length
    requirements.push(length_rule_to_must_should(
        "summary",
        deny_subject_length,
        warn_subject_length,
    ));

    // body presence
    requirements.push(negative_rule_to_must_should(
        "message",
        "body",
        deny_no_body,
        warn_no_body,
    ));

    // body length
    requirements.push(length_rule_to_must_should(
        "message body",
        deny_body_length,
        warn_body_length,
    ));

    // breaking_change_footer presence
    requirements.push(negative_rule_to_must_should(
        "footers",
        r#""BREAKING CHANGE:" footer"#,
        deny_no_breaking_change_footer,
        warn_no_breaking_change_footer,
    ));

    // final requirements
    requirements.append(&mut vec![
        "- use lowercase for the summary".into(),
        "- be concise and descriptive".into(),
        "- lowercase bullet points are preferred over paragraphs for message body".into(),
        r#"- focus more on "why" than "what" and "how""#.into(),
        "- respond with only the commit message, nothing else".into(),
    ]);

    let requirements_paragraph = requirements.join("\n");

    [
        intro_paragraph.into(),
        diff_paragraph,
        branch_paragraph,
        recent_commits_paragraph,
        requirements_paragraph,
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n")
}

fn set_to_list_string(set: &HashSet<String>) -> String {
    set.iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

fn length_rule_to_must_should(name: &str, deny: Option<usize>, warn: Option<usize>) -> String {
    match (deny, warn) {
        (Some(deny_length), None) => format!("- {name} MUST be {deny_length} characters or less"),
        (None, Some(warn_length)) => {
            format!("- {name} should be {warn_length} characters or less")
        }
        (Some(deny_length), Some(warn_length)) => {
            format!("- {name} MUST be {deny_length} characters or less, {warn_length} preferred")
        }
        _ => String::new(),
    }
}

/// Translates a rule like `no_scope` into one of:
/// - "{area} MUST have {name}", or
/// - "{area} should have {name}"
fn negative_rule_to_must_should(
    area: &str,
    name: &str,
    deny: Option<bool>,
    warn: Option<bool>,
) -> String {
    if deny.unwrap_or(false) {
        format!("{area} MUST have {name}")
    } else if warn.unwrap_or(false) {
        format!("{area} should have {area}")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    // serialise tests that mutate env vars to prevent parallel-test race conditions
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_build_prompt_with_context() {
        let _guard = ENV_LOCK.lock().unwrap();

        let context = CommitContext {
            branch_name: Some("feature/test".to_string()),
            recent_commits: vec!["feat: previous commit".to_string()],
            repository_name: Some("test-repo".to_string()),
            is_merge: false,
            is_rebase: false,
        };

        let prompt = build_prompt("test diff", &context, &CommitConfig::default());

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert!(prompt.contains("test diff"));
        assert!(prompt.contains("feature/test"));
        assert!(prompt.contains("feat: previous commit"));
        assert!(prompt.contains("conventional commits"));
    }

    #[test]
    fn test_build_prompt_without_branch() {
        let _guard = ENV_LOCK.lock().unwrap();

        let context = CommitContext::default();

        let prompt = build_prompt("test diff", &context, &CommitConfig::default());

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert!(prompt.contains("test diff"));
        assert!(!prompt.contains("branch name:"));
    }
}
