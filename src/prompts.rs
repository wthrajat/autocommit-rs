pub const FALLBACK_MESSAGE: &str = "chore(scope): update files";

pub const SYSTEM_PROMPT_SHORT: &str = "You are a git commit generator. Follow Conventional Commits strictly.

Rules:
1. Output EXACTLY ONE summary line only.
2. ALWAYS include scope like feat(auth): or fix(core):.
3. Summary max 72 chars, lowercase, no trailing period.
4. NEVER output multiple separate commits, combine them into one.";

pub const SYSTEM_PROMPT_LONG: &str = "You are a git commit generator. Follow Conventional Commits strictly.

Rules:
1. Output EXACTLY ONE summary line first.
2. ALWAYS include scope like feat(auth): or fix(core):.
3. Summary max 72 chars, lowercase, no trailing period.
4. Add ONE blank line after summary, then bullet points with \"-\" for each change.
5. DO NOT use markdown code blocks.
6. NEVER output multiple separate commits, combine them into one.";

pub const MAX_DIFF_LENGTH: usize = 10_000;
pub const MAX_TOKENS_SHORT: u32 = 60;
pub const MAX_TOKENS_LONG: u32 = 150;
