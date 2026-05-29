use super::sequence_operations::finalize_sequence_commit;
use crate::git_manager::core::operations_history_rewrite_support::{
    cleanup_repository_state, collect_conflict_paths, ensure_active_cherry_pick_state,
    ensure_clean_repository_state, history_result, is_cherry_pick_state, is_conflict_error_code,
    non_empty, read_sequence_message, reset_head_hard, resolve_commit,
    resolve_sequence_head_commit,
};
use crate::git_manager::core::{
    CherryPickRequest, GitError, GitErrorCode, GitResult, HistoryRewriteResult, HistoryRewriteState,
};
use git2::{CherrypickOptions, Repository};

pub(super) fn execute_cherry_pick_operation(
    repository: &Repository,
    request: &CherryPickRequest,
) -> GitResult<HistoryRewriteResult> {
    ensure_clean_repository_state(repository, "cherry-pick")?;

    let source_commit = resolve_commit(
        repository,
        request.commit.as_str(),
        "history_rewrite.cherry_pick.commit",
    )?;

    let mut options = CherrypickOptions::new();
    if let Some(mainline) = request.mainline {
        options.mainline(mainline);
    }

    if let Err(error) = repository.cherrypick(&source_commit, Some(&mut options)) {
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
                "cherry-pick failed to apply commit `{}`: {error}",
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

    let message = source_commit.message().map_or_else(
        || format!("cherry-pick {}", source_commit.id()),
        str::to_owned,
    );
    let author = source_commit.author();
    finalize_sequence_commit(repository, Some(&author), message.as_str(), "cherry-pick")?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Completed,
        Vec::new(),
    ))
}

pub(super) fn execute_cherry_pick_continue(
    repository: &Repository,
) -> GitResult<HistoryRewriteResult> {
    ensure_active_cherry_pick_state(repository, "cherry-pick.continue")?;

    let source_commit =
        resolve_sequence_head_commit(repository, "CHERRY_PICK_HEAD", "cherry-pick.continue")?;
    let fallback_message = source_commit.as_ref().map_or_else(
        || "cherry-pick".to_string(),
        |commit| {
            commit
                .message()
                .and_then(non_empty)
                .map_or_else(|| format!("cherry-pick {}", commit.id()), str::to_owned)
        },
    );
    let message = read_sequence_message(
        repository,
        "cherry-pick.continue",
        fallback_message.as_str(),
    )?;
    let author = source_commit.as_ref().map(git2::Commit::author);
    finalize_sequence_commit(
        repository,
        author.as_ref(),
        message.as_str(),
        "cherry-pick.continue",
    )?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Continued,
        Vec::new(),
    ))
}

pub(super) fn execute_cherry_pick_abort(
    repository: &Repository,
) -> GitResult<HistoryRewriteResult> {
    if !is_cherry_pick_state(repository.state()) {
        return Ok(history_result(
            repository,
            HistoryRewriteState::Aborted,
            Vec::new(),
        ));
    }

    cleanup_repository_state(repository, "cherry-pick.abort")?;
    reset_head_hard(repository, "cherry-pick.abort")?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Aborted,
        Vec::new(),
    ))
}

pub(super) fn execute_cherry_pick_skip(repository: &Repository) -> GitResult<HistoryRewriteResult> {
    ensure_active_cherry_pick_state(repository, "cherry-pick.skip")?;

    cleanup_repository_state(repository, "cherry-pick.skip")?;
    reset_head_hard(repository, "cherry-pick.skip")?;

    Ok(history_result(
        repository,
        HistoryRewriteState::Skipped,
        Vec::new(),
    ))
}
