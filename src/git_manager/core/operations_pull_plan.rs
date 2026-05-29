use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, PullRequest, RepositoryDescriptor,
};
use git2::{ErrorCode, Repository};

pub(super) fn validate_pull_request(request: &PullRequest) -> GitResult<()> {
    if request.remote_name.trim().is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "pull operation requires a non-empty remote name",
        ));
    }

    Ok(())
}

pub(super) fn resolve_pull_branch(
    descriptor: &RepositoryDescriptor,
    request: &PullRequest,
    current_branch_name: &str,
) -> GitResult<String> {
    if let Some(requested_branch) = request.branch.as_deref().and_then(non_empty) {
        return Ok(requested_branch.to_owned());
    }

    let upstream_name = descriptor.upstream_branch.as_deref().ok_or_else(|| {
        GitError::new(
            GitErrorCode::UpstreamNotFound,
            format!(
                "failed to resolve upstream for local branch `{current_branch_name}` via repository descriptor"
            ),
        )
    })?;

    parse_upstream_branch_name(upstream_name, request.remote_name.as_str()).ok_or_else(|| {
        GitError::new(
            GitErrorCode::UpstreamNotFound,
            format!(
                "upstream `{upstream_name}` does not map to remote `{}`",
                request.remote_name
            ),
        )
    })
}

fn parse_upstream_branch_name(upstream_name: &str, remote_name: &str) -> Option<String> {
    let remote_name = non_empty(remote_name)?;

    let refs_prefix = format!("refs/remotes/{remote_name}/");
    if let Some(branch_name) = upstream_name.strip_prefix(&refs_prefix) {
        return non_empty(branch_name).map(str::to_owned);
    }

    let shorthand_prefix = format!("{remote_name}/");
    if let Some(branch_name) = upstream_name.strip_prefix(&shorthand_prefix) {
        return non_empty(branch_name).map(str::to_owned);
    }

    None
}

pub(super) fn ensure_pull_rebase_disabled(
    repository: &Repository,
    current_branch: &str,
) -> GitResult<()> {
    let config = repository.config().map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("failed to open repository config for pull.rebase guard: {error}"),
        )
    })?;

    let branch_rebase_key = format!("branch.{current_branch}.rebase");
    if config_key_truthy(&config, &branch_rebase_key)? || config_key_truthy(&config, "pull.rebase")?
    {
        return Err(GitError::new(
            GitErrorCode::PullRebaseNotSupportedYet,
            "pull.rebase=true is configured, but pull rebase mode is not supported yet",
        ));
    }

    Ok(())
}

fn config_key_truthy(config: &git2::Config, key: &str) -> GitResult<bool> {
    if let Ok(value) = config.get_bool(key) {
        return Ok(value);
    }

    match config.get_string(key) {
        Ok(raw_value) => Ok(parse_rebase_truthy(&raw_value)),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(false),
        Err(error) => Err(config_read_error(key, &error)),
    }
}

fn parse_rebase_truthy(raw_value: &str) -> bool {
    let normalized = raw_value.trim().to_ascii_lowercase();
    !normalized.is_empty() && !matches!(normalized.as_str(), "0" | "false" | "no" | "off")
}

fn config_read_error(key: &str, error: &git2::Error) -> GitError {
    GitError::new(
        GitErrorCode::Internal,
        format!("failed to read git config key `{key}` for pull guard: {error}"),
    )
}

pub(super) fn detached_head_error() -> GitError {
    GitError::new(
        GitErrorCode::DetachedHead,
        "pull operation requires an attached branch HEAD",
    )
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}
