use crate::git_manager::core::operations_query_lifecycle_support::{
    classify_unsupported_command, empty_query_result, map_query_error, normalize_non_empty,
    validate_blame_line_range,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, QueryBlameHunk, QueryConfigValue, QueryLifecycleResult,
    QueryVersionInfo,
};
use git2::{
    Blame, BlameHunk, BlameIter, BlameOptions, ErrorCode, Repository, RepositoryInitOptions,
};
use std::path::Path;

pub(super) fn execute_blame_like_operation(
    repository: &Repository,
    path: &str,
    min_line: Option<usize>,
    max_line: Option<usize>,
    use_mailmap: bool,
) -> GitResult<QueryLifecycleResult> {
    let path = normalize_non_empty(path, "query_lifecycle.blame.path")?;
    validate_blame_line_range(min_line, max_line)?;
    let mut blame_options = BlameOptions::new();
    blame_options.use_mailmap(use_mailmap);
    if let Some(min_line) = min_line {
        blame_options.min_line(min_line);
    }
    if let Some(max_line) = max_line {
        blame_options.max_line(max_line);
    }
    let blame: Blame<'_> = repository
        .blame_file(Path::new(path), Some(&mut blame_options))
        .map_err(|error| {
            map_query_error(
                &error,
                "query_lifecycle.blame failed to collect blame hunks",
            )
        })?;
    let blame_iter: BlameIter<'_> = blame.iter();
    let mut result = empty_query_result();
    result.blame_hunks = blame_iter
        .map(|hunk: BlameHunk<'_>| {
            let signature = hunk.final_signature();
            QueryBlameHunk {
                final_commit_oid: hunk.final_commit_id().to_string(),
                final_start_line: hunk.final_start_line(),
                lines_in_hunk: hunk.lines_in_hunk(),
                source_path: hunk
                    .path()
                    .map(|source_path| source_path.display().to_string()),
                author_name: signature.name().map(str::to_owned),
                author_email: signature.email().map(str::to_owned),
            }
        })
        .collect();
    Ok(result)
}

pub(super) fn execute_config_get_operation(
    repository: &Repository,
    key: &str,
) -> GitResult<QueryLifecycleResult> {
    let key = normalize_non_empty(key, "query_lifecycle.config_get.key")?;
    let config = repository.config().map_err(|error| {
        map_query_error(
            &error,
            "query_lifecycle.config_get failed to open repository config",
        )
    })?;
    let value = match config.get_string(key) {
        Ok(value) => Some(value),
        Err(error) if error.code() == ErrorCode::NotFound => None,
        Err(error) => {
            return Err(map_query_error(
                &error,
                format!("query_lifecycle.config_get failed to read config key `{key}`"),
            ));
        }
    };
    let mut result = empty_query_result();
    result.config_value = Some(QueryConfigValue {
        key: key.to_owned(),
        value,
    });
    Ok(result)
}

pub(super) fn execute_init_operation(
    repository_path: &Path,
    bare: bool,
    initial_branch: Option<&str>,
) -> GitResult<QueryLifecycleResult> {
    if repository_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            "query_lifecycle.init requires a non-empty repository path",
        ));
    }

    let initial_branch = initial_branch
        .map(|branch| normalize_non_empty(branch, "query_lifecycle.init.initial_branch"))
        .transpose()?;

    #[cfg(all(debug_assertions, feature = "trace_logs"))]
    tracing::trace!(
        repository = %repository_path.display(),
        bare,
        initial_branch = initial_branch.unwrap_or(""),
        "query_lifecycle_init_requested"
    );

    let mut init_options = RepositoryInitOptions::new();
    init_options.bare(bare);
    if let Some(initial_branch) = initial_branch {
        init_options.initial_head(initial_branch);
    }
    let initialized_repository =
        Repository::init_opts(repository_path, &init_options).map_err(|error| {
            map_query_error(
                &error,
                format!(
                    "query_lifecycle.init failed for `{}`",
                    repository_path.display()
                ),
            )
        })?;

    #[cfg(all(debug_assertions, feature = "trace_logs"))]
    tracing::trace!(
        repository = %repository_path.display(),
        git_dir = %initialized_repository.path().display(),
        "query_lifecycle_init_succeeded"
    );

    let mut result = empty_query_result();
    result.changed = true;
    result.initialized_repository = Some(initialized_repository.path().to_path_buf());
    result.summary = Some(if bare {
        "initialized bare repository".to_string()
    } else {
        "initialized repository".to_string()
    });
    Ok(result)
}

pub(super) fn execute_unsupported_command_operation(
    command: &str,
) -> GitResult<QueryLifecycleResult> {
    let unsupported = classify_unsupported_command(command)?;
    let mut result = empty_query_result();
    result.summary = Some(format!(
        "explicit unsupported classification for `{}`",
        unsupported.command
    ));
    result.unsupported = Some(unsupported);
    Ok(result)
}

#[must_use]
pub(super) fn execute_version_operation() -> QueryLifecycleResult {
    let version = git2::Version::get();
    let mut result = empty_query_result();
    result.version = Some(QueryVersionInfo {
        git2_crate_version: version.crate_version().to_string(),
        libgit2_version: version.libgit2_version(),
        vendored: version.vendored(),
        threads: version.threads(),
        https: version.https(),
        ssh: version.ssh(),
        nsec: version.nsec(),
    });
    result.summary = Some("git2/libgit2 version snapshot".to_string());
    result
}
