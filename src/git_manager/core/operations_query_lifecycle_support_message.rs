use super::{map_query_error, normalize_non_empty};
use crate::git_manager::core::query_lifecycle_contracts::{
    QueryMessageDetails, QueryMessageTrailerBytes, QueryMessageTrailerStr, QueryObjectInfo,
    QueryRevspec, QueryRevspecMode, QueryUnsupportedClassification,
};
use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{message_prettify, message_trailers_bytes, message_trailers_strs, Repository};

const QUERY_UNSUPPORTED_REASON_CODE: &str = "COMMAND_NOT_EXPOSED_IN_GITMANAGER_API";
const QUERY_UNSUPPORTED_COMMANDS: [&str; 13] = [
    "bisect",
    "bugreport",
    "column",
    "diagnose",
    "grep",
    "help",
    "instaweb",
    "sh-i18n",
    "sh-setup",
    "shell",
    "var",
    "verify-commit",
    "verify-tag",
];

pub(crate) fn resolve_revspec(repository: &Repository, spec: &str) -> GitResult<QueryRevspec> {
    let spec = normalize_non_empty(spec, "query_lifecycle.revparse.spec")?;
    let revspec: git2::Revspec<'_> = repository.revparse(spec).map_err(|error| {
        map_query_error(
            &error,
            format!("query_lifecycle.revparse failed to parse spec `{spec}`"),
        )
    })?;
    let mode = revspec.mode();

    Ok(QueryRevspec {
        spec: spec.to_owned(),
        from: revspec.from().map(map_object_info),
        to: revspec.to().map(map_object_info),
        mode: QueryRevspecMode {
            single: mode.contains(git2::RevparseMode::SINGLE),
            range: mode.contains(git2::RevparseMode::RANGE),
            merge_base: mode.contains(git2::RevparseMode::MERGE_BASE),
        },
    })
}

pub(crate) fn extract_message_details(
    message: &str,
    context: &str,
) -> GitResult<QueryMessageDetails> {
    let prettified_message =
        message_prettify(message, git2::DEFAULT_COMMENT_CHAR).map_err(|error| {
            map_query_error(
                &error,
                format!("{context} failed to prettify commit message"),
            )
        })?;

    let trailers_strs: git2::MessageTrailersStrs =
        message_trailers_strs(prettified_message.as_str()).map_err(|error| {
            map_query_error(
                &error,
                format!("{context} failed to parse trailers as UTF-8"),
            )
        })?;
    let trailers_strs_iter: git2::MessageTrailersStrsIterator<'_> = trailers_strs.iter();
    let trailers_strs = trailers_strs_iter
        .map(|(key, value)| QueryMessageTrailerStr {
            key: key.to_owned(),
            value: value.to_owned(),
        })
        .collect();

    let trailers_bytes: git2::MessageTrailersBytes =
        message_trailers_bytes(prettified_message.as_bytes()).map_err(|error| {
            map_query_error(
                &error,
                format!("{context} failed to parse trailers as bytes"),
            )
        })?;
    let trailers_bytes_iter: git2::MessageTrailersBytesIterator<'_> = trailers_bytes.iter();
    let trailers_bytes = trailers_bytes_iter
        .map(|(key, value)| QueryMessageTrailerBytes {
            key: key.to_vec(),
            value: value.to_vec(),
        })
        .collect();

    Ok(QueryMessageDetails {
        prettified_message,
        default_comment_char: git2::DEFAULT_COMMENT_CHAR,
        trailers_strs,
        trailers_bytes,
    })
}

pub(crate) fn validate_blame_line_range(
    min_line: Option<usize>,
    max_line: Option<usize>,
) -> GitResult<()> {
    if matches!(min_line, Some(0)) {
        return Err(GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            "query_lifecycle blame min_line must be greater than zero",
        ));
    }

    if matches!(max_line, Some(0)) {
        return Err(GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            "query_lifecycle blame max_line must be greater than zero",
        ));
    }

    if let (Some(min_line), Some(max_line)) = (min_line, max_line) {
        if max_line < min_line {
            return Err(GitError::new(
                GitErrorCode::QueryLifecycleInvalidInput,
                "query_lifecycle blame max_line must be greater than or equal to min_line",
            ));
        }
    }

    Ok(())
}

pub(crate) fn classify_unsupported_command(
    command: &str,
) -> GitResult<QueryUnsupportedClassification> {
    let command = normalize_non_empty(command, "query_lifecycle.unsupported_command.command")?;
    let normalized_command = command.to_ascii_lowercase();

    if !QUERY_UNSUPPORTED_COMMANDS.contains(&normalized_command.as_str()) {
        return Err(GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            format!(
                "query_lifecycle.unsupported_command.command `{command}` is not in T-086.13 unsupported catalog"
            ),
        ));
    }

    let impact = format!(
        "The `{normalized_command}` command has no typed operation in `GitManager::query_lifecycle`; running it through GitManager is unavailable without a dedicated API contract."
    );
    Ok(QueryUnsupportedClassification {
        command: normalized_command,
        reason_code: QUERY_UNSUPPORTED_REASON_CODE.to_string(),
        impact,
    })
}

fn map_object_info(object: &git2::Object<'_>) -> QueryObjectInfo {
    QueryObjectInfo {
        object_id: object.id().to_string(),
        kind: object
            .kind()
            .map_or_else(|| "unknown".to_string(), |kind| kind.str().to_string()),
    }
}
