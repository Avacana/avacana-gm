use super::{empty_advanced_result, map_advanced_error, normalize_non_empty};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};

pub(crate) fn execute_trace_set_operation(level: &str) -> GitResult<AdvancedResult> {
    let level = normalize_non_empty(level, "advanced.trace_set.level")?;
    let trace_level = parse_trace_level(level)?;

    git2::trace_set(trace_level, advanced_trace_callback).map_err(|error| {
        map_advanced_error(
            &error,
            "advanced.trace_set failed to register libgit2 callback",
        )
    })?;

    let mut result = empty_advanced_result();
    result.changed = true;
    result.summary = Some(format!("libgit2 trace level set to `{level}`"));
    result.items.push(format!("level:{level}"));
    Ok(result)
}

fn parse_trace_level(level: &str) -> GitResult<git2::TraceLevel> {
    match level.to_ascii_lowercase().as_str() {
        "none" => Ok(git2::TraceLevel::None),
        "fatal" => Ok(git2::TraceLevel::Fatal),
        "error" => Ok(git2::TraceLevel::Error),
        "warn" => Ok(git2::TraceLevel::Warn),
        "info" => Ok(git2::TraceLevel::Info),
        "debug" => Ok(git2::TraceLevel::Debug),
        "trace" => Ok(git2::TraceLevel::Trace),
        _ => Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            "advanced.trace_set.level must be one of: none,fatal,error,warn,info,debug,trace",
        )),
    }
}

const fn advanced_trace_callback(_level: git2::TraceLevel, _message: &[u8]) {}
