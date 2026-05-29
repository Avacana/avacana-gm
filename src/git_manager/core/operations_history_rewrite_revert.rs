use super::sequence_operations::finalize_sequence_commit;
use crate::git_manager::core::operations_history_rewrite_support::{
    cleanup_repository_state, collect_conflict_paths, ensure_active_revert_state,
    ensure_clean_repository_state, history_result, is_conflict_error_code, is_revert_state,
    non_empty, read_sequence_message, reset_head_hard, resolve_commit,
    resolve_sequence_head_commit,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, HistoryRewriteResult, HistoryRewriteState, RevertRequest,
};
use git2::{Repository, RevertOptions};

pub(super) fn execute_revert_operation(
    repository: &Repository,
    request: &RevertRequest,
) -> GitResult<HistoryRewriteResult> {
    ensure_clean_repository_state(repository, "revert")?;

    let source_commit = resolve_commit(
        repository,
        request.commit.as_str(),
        "history_rewrite.revert.commit",
    )?;

    let mut options = RevertOptions::new();
    if let Some(mainline) = request.mainline {
        options.mainline(mainline);
    }

    if let Err(error) = repository.revert(&source_commit, Some(&mut options)) {
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
            format!(
                "revert failed to apply commit `{}`: {error}",
                source_commit.id()
            ),
        ));
    }

    let conflict_paths = collect_conflict_paths(repository)?;
    if !conflict_paths.is_empty() {
        return Ok(history_result(
            repository,
            HistoryRewriteState::Conflict,
            conflict_paths,
        ));
    }

    let summary = source_commit
        .summary()
        .and_then(non_empty)
        .unwrap_or("commit");
    let message = format!(
        "Revert \"{summary}\"\n\nThis reverts commit {}.",
        source_commit.id()
    );
    finalize_sequence_commit(repository, None, message.as_str(), "revert")?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Completed,
        Vec::new(),
    ))
}

pub(super) fn execute_revert_continue(repository: &Repository) -> GitResult<HistoryRewriteResult> {
    ensure_active_revert_state(repository, "revert.continue")?;

    let source_commit = resolve_sequence_head_commit(repository, "REVERT_HEAD", "revert.continue")?;
    let fallback_message = source_commit.as_ref().map_or_else(
        || "revert".to_string(),
        |commit| {
            let summary = commit.summary().and_then(non_empty).unwrap_or("commit");
            format!(
                "Revert \"{summary}\"\n\nThis reverts commit {}.",
                commit.id()
            )
        },
    );
    let message = read_sequence_message(repository, "revert.continue", fallback_message.as_str())?;
    finalize_sequence_commit(repository, None, message.as_str(), "revert.continue")?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Continued,
        Vec::new(),
    ))
}

pub(super) fn execute_revert_abort(repository: &Repository) -> GitResult<HistoryRewriteResult> {
    if !is_revert_state(repository.state()) {
        return Ok(history_result(
            repository,
            HistoryRewriteState::Aborted,
            Vec::new(),
        ));
    }

    cleanup_repository_state(repository, "revert.abort")?;
    reset_head_hard(repository, "revert.abort")?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Aborted,
        Vec::new(),
    ))
}

pub(super) fn execute_revert_skip(repository: &Repository) -> GitResult<HistoryRewriteResult> {
    ensure_active_revert_state(repository, "revert.skip")?;

    cleanup_repository_state(repository, "revert.skip")?;
    reset_head_hard(repository, "revert.skip")?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Skipped,
        Vec::new(),
    ))
}
