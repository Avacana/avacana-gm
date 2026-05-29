use crate::git_manager::core::operations_history_rewrite_support::{
    cleanup_repository_state, collect_conflict_paths, ensure_active_rebase_state,
    ensure_clean_repository_state, history_result, is_conflict_error_code, is_rebase_state,
    open_active_rebase, reset_head_hard, resolve_annotated_commit,
    resolve_optional_annotated_commit, resolve_signature,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, HistoryRewriteResult, HistoryRewriteState, RebaseAction,
    RebaseRequest,
};
use git2::{
    ErrorCode, Rebase, RebaseOperation, RebaseOperationType, RebaseOptions, Repository,
    RepositoryState,
};

pub(super) fn execute_rebase_operation(
    repository: &Repository,
    request: &RebaseRequest,
) -> GitResult<HistoryRewriteResult> {
    match request.action {
        RebaseAction::Start => execute_rebase_start(repository, request),
        RebaseAction::Continue => execute_rebase_continue(repository),
        RebaseAction::Abort => execute_rebase_abort(repository),
        RebaseAction::Skip => execute_rebase_skip(repository),
    }
}

fn execute_rebase_start(
    repository: &Repository,
    request: &RebaseRequest,
) -> GitResult<HistoryRewriteResult> {
    ensure_clean_repository_state(repository, "rebase.start")?;

    let upstream_spec = request
        .upstream
        .as_deref()
        .and_then(crate::git_manager::core::operations_history_rewrite_support::non_empty)
        .ok_or_else(|| {
            GitError::new(
                GitErrorCode::RefNotFound,
                "history_rewrite.rebase.upstream must be provided for start action",
            )
        })?;
    let branch = resolve_optional_annotated_commit(
        repository,
        request.branch.as_deref(),
        "history_rewrite.rebase.branch",
    )?;
    let upstream =
        resolve_annotated_commit(repository, upstream_spec, "history_rewrite.rebase.upstream")?;
    let onto = resolve_optional_annotated_commit(
        repository,
        request.onto.as_deref(),
        "history_rewrite.rebase.onto",
    )?;

    let mut rebase_options = RebaseOptions::new();
    rebase_options.quiet(true).inmemory(false);
    let mut rebase = repository
        .rebase(
            branch.as_ref(),
            Some(&upstream),
            onto.as_ref(),
            Some(&mut rebase_options),
        )
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("rebase.start failed to initialize rebase operation: {error}"),
            )
        })?;

    drive_rebase_lifecycle(
        repository,
        &mut rebase,
        HistoryRewriteState::Completed,
        "rebase.start",
    )
}

fn execute_rebase_continue(repository: &Repository) -> GitResult<HistoryRewriteResult> {
    ensure_active_rebase_state(repository, "rebase.continue")?;

    let mut rebase = open_active_rebase(repository, "rebase.continue")?;
    if rebase.operation_current().is_some() {
        if let Some(conflict_result) =
            commit_current_rebase_patch(repository, &mut rebase, "rebase.continue")?
        {
            return Ok(conflict_result);
        }
    }

    drive_rebase_lifecycle(
        repository,
        &mut rebase,
        HistoryRewriteState::Continued,
        "rebase.continue",
    )
}

fn execute_rebase_abort(repository: &Repository) -> GitResult<HistoryRewriteResult> {
    if !is_rebase_state(repository.state()) {
        return Ok(history_result(
            repository,
            HistoryRewriteState::Aborted,
            Vec::new(),
        ));
    }

    let mut rebase = open_active_rebase(repository, "rebase.abort")?;
    rebase.abort().map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("rebase.abort failed to abort active rebase state: {error}"),
        )
    })?;

    if repository.state() != RepositoryState::Clean {
        cleanup_repository_state(repository, "rebase.abort")?;
    }

    Ok(history_result(
        repository,
        HistoryRewriteState::Aborted,
        Vec::new(),
    ))
}

fn execute_rebase_skip(repository: &Repository) -> GitResult<HistoryRewriteResult> {
    ensure_active_rebase_state(repository, "rebase.skip")?;

    let mut rebase = open_active_rebase(repository, "rebase.skip")?;
    if rebase.operation_current().is_some() {
        reset_head_hard(repository, "rebase.skip")?;
    }

    drive_rebase_lifecycle(
        repository,
        &mut rebase,
        HistoryRewriteState::Skipped,
        "rebase.skip",
    )
}

fn drive_rebase_lifecycle(
    repository: &Repository,
    rebase: &mut Rebase<'_>,
    completed_state: HistoryRewriteState,
    context: &str,
) -> GitResult<HistoryRewriteResult> {
    loop {
        match rebase.next() {
            Some(Ok(operation)) => {
                let operation: RebaseOperation<'_> = operation;
                let operation_kind: Option<RebaseOperationType> = operation.kind();
                tracing::trace!(
                    context = context,
                    operation_kind = ?operation_kind,
                    operation_commit = ?operation.id(),
                    "rebase advanced to next operation"
                );

                let conflict_paths = collect_conflict_paths(repository)?;
                if !conflict_paths.is_empty() {
                    return Ok(history_result(
                        repository,
                        HistoryRewriteState::Conflict,
                        conflict_paths,
                    ));
                }

                if let Some(conflict_result) =
                    commit_current_rebase_patch(repository, rebase, context)?
                {
                    return Ok(conflict_result);
                }
            }
            Some(Err(error)) => {
                let conflict_paths = collect_conflict_paths(repository)?;
                if !conflict_paths.is_empty() || is_conflict_error_code(error.code()) {
                    return Ok(history_result(
                        repository,
                        HistoryRewriteState::Conflict,
                        conflict_paths,
                    ));
                }

                return Err(GitError::new(
                    GitErrorCode::Internal,
                    format!("{context} failed to advance rebase operation: {error}"),
                ));
            }
            None => {
                finish_rebase(repository, rebase, context)?;
                return Ok(history_result(repository, completed_state, Vec::new()));
            }
        }
    }
}

fn commit_current_rebase_patch(
    repository: &Repository,
    rebase: &mut Rebase<'_>,
    context: &str,
) -> GitResult<Option<HistoryRewriteResult>> {
    let signature = resolve_signature(repository, context)?;
    match rebase.commit(None, &signature, None) {
        Ok(commit_oid) => {
            tracing::trace!(
                context = context,
                committed_oid = %commit_oid,
                "rebase committed current operation"
            );
            Ok(None)
        }
        Err(error) if error.code() == ErrorCode::Applied => {
            tracing::trace!(
                context = context,
                "rebase commit skipped because patch is already applied"
            );
            Ok(None)
        }
        Err(error) => {
            let conflict_paths = collect_conflict_paths(repository)?;
            if !conflict_paths.is_empty() || is_conflict_error_code(error.code()) {
                return Ok(Some(history_result(
                    repository,
                    HistoryRewriteState::Conflict,
                    conflict_paths,
                )));
            }

            Err(GitError::new(
                GitErrorCode::Internal,
                format!("{context} failed to commit rebase operation: {error}"),
            ))
        }
    }
}

fn finish_rebase(repository: &Repository, rebase: &mut Rebase<'_>, context: &str) -> GitResult<()> {
    let signature = resolve_signature(repository, context)?;
    rebase.finish(Some(&signature)).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("{context} failed to finish rebase operation: {error}"),
        )
    })
}
