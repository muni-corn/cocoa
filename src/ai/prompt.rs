use crate::ai::client::CommitContext;

pub fn build_system_prompt() -> String {
    "you are an expert software engineer who writes excellent conventional commit messages. respond with only the commit message and body, no explanation or additional text.".to_string()
}

/// Builds the prompt for commit generation.
pub fn build_prompt(staged_changes: &str, context: &CommitContext) -> String {
    let mut prompt = String::new();

    prompt.push_str("generate a conventional commit message for these staged changes:\n\n");
    prompt.push_str("```diff\n");
    prompt.push_str(staged_changes);
    prompt.push_str("\n```\n\n");

    if let Some(branch) = &context.branch_name {
        prompt.push_str(&format!("branch name: {}\n\n", branch));
    }

    if !context.recent_commits.is_empty() {
        prompt.push_str("recent commit messages for context:\n");
        for commit in context.recent_commits.iter().take(5) {
            prompt.push_str(&format!("- {}\n", commit));
        }
        prompt.push('\n');
    }

    prompt.push_str("requirements:\n");
    prompt.push_str("- use conventional commits format (type: description)\n");
    prompt.push_str("- type must be one of: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert\n");
    prompt.push_str("- keep subject line under 50 characters if possible\n");
    prompt.push_str("- use lowercase for the description\n");
    prompt.push_str("- be concise and descriptive\n");
    prompt.push_str("- respond with only the commit message, nothing else\n");

    prompt
}
