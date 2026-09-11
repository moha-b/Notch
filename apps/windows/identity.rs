// Shared by the app and dependency-free hook. Stable is set only by release stamping.
pub const STABLE: bool = cfg!(feature = "stable-channel");
pub const NAMESPACE: &str = if STABLE { "io.github.moha-b.notch" } else { "io.github.moha-b.notch.preview" };
pub const NAME: &str = if STABLE { "Notch" } else { "Notch Preview" };
pub const PORT: u16 = if STABLE { 48766 } else { 48767 };

/// A Claude profile id that is safe on a hook command line: `claude-` plus ASCII letters, digits, `.`, `_` or `-`.
pub fn is_hook_profile_id(id: &str) -> bool {
    id.len() <= 64
        && id.strip_prefix("claude-").is_some_and(|slug| {
            !slug.is_empty() && slug.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}
