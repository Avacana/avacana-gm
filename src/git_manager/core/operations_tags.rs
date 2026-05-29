//! `tag_summaries` operation for `GitManager`.

use crate::git_manager::core::repository_access::open_repository_context;
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, TagSummariesRequest, TagSummariesResult, TagSummary,
};
use git2::{ErrorCode, ObjectType, Repository};
use std::time::Instant;

/// Performs a typed read-only read of tags whose final target is a commit.
///
/// # Errors
/// Returns a typed `GitError` if the repository cannot be opened, the tags cannot be enumerated,
/// or a tag reference cannot be correctly peeled to its final object. Tags pointing at tree/blob and
/// other non-commit objects are gracefully skipped without error.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            operation = "tag_summaries",
            requested_path = %request.repository_path.display(),
            repo_root = tracing::field::Empty,
            tag_count = tracing::field::Empty,
            skipped_non_commit_tags = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty
        )
    )
)]
pub(super) fn execute_tag_summaries_operation(
    request: &TagSummariesRequest,
) -> GitResult<TagSummariesResult> {
    let started_at = Instant::now();
    let opened_repository = open_repository_context(&request.repository_path, "tag_summaries")?;
    let collected = collect_tag_summaries(&opened_repository.repository)?;
    let elapsed_ms = started_at.elapsed().as_millis();

    tracing::Span::current().record(
        "repo_root",
        tracing::field::display(opened_repository.repo_root.display()),
    );
    tracing::Span::current().record("tag_count", tracing::field::display(collected.tags.len()));
    tracing::Span::current().record(
        "skipped_non_commit_tags",
        tracing::field::display(collected.skipped_non_commit_tags),
    );
    tracing::Span::current().record("elapsed_ms", tracing::field::display(elapsed_ms));
    tracing::trace!(
        operation = "tag_summaries",
        requested_path = %request.repository_path.display(),
        repo_root = %opened_repository.repo_root.display(),
        tag_count = collected.tags.len(),
        skipped_non_commit_tags = collected.skipped_non_commit_tags,
        elapsed_ms,
        "resolved typed tag summaries"
    );

    Ok(TagSummariesResult::new(collected.tags))
}

struct CollectedTagSummaries {
    tags: Vec<TagSummary>,
    skipped_non_commit_tags: usize,
}

fn collect_tag_summaries(repository: &Repository) -> GitResult<CollectedTagSummaries> {
    let tag_names = repository.tag_names(None).map_err(|error| {
        map_tag_summaries_error(&error, "tag_summaries failed to enumerate repository tags")
    })?;

    let mut tags = Vec::new();
    let mut skipped_non_commit_tags = 0_usize;

    for short_name in tag_names.iter().flatten() {
        let reference_name = format!("refs/tags/{short_name}");
        let reference = repository
            .find_reference(&reference_name)
            .map_err(|error| {
                map_tag_summaries_error(
                    &error,
                    format!("tag_summaries failed to resolve reference `{reference_name}`"),
                )
            })?;

        let peeled_target = reference.peel(ObjectType::Any).map_err(|error| {
            map_tag_summaries_error(
                &error,
                format!("tag_summaries failed to peel reference `{reference_name}`"),
            )
        })?;
        let target_kind = peeled_target.kind().ok_or_else(|| {
            GitError::new(
                GitErrorCode::TagSummariesFailed,
                format!(
                    "tag_summaries failed to determine final target kind for reference `{reference_name}`"
                ),
            )
        })?;
        if target_kind != ObjectType::Commit {
            skipped_non_commit_tags += 1;
            tracing::trace!(
                reference_name,
                ?target_kind,
                "skipping tag whose final target is not a commit"
            );
            continue;
        }
        let commit = peeled_target.peel_to_commit().map_err(|error| {
            map_tag_summaries_error(
                &error,
                format!(
                    "tag_summaries failed to peel final target of `{reference_name}` to commit"
                ),
            )
        })?;

        tags.push(TagSummary {
            reference_name,
            short_name: short_name.to_owned(),
            target_commit_oid: commit.id().to_string(),
            target_commit_timestamp: commit.time().seconds(),
        });
    }

    tags.sort_by(|left, right| left.reference_name.cmp(&right.reference_name));

    Ok(CollectedTagSummaries {
        tags,
        skipped_non_commit_tags,
    })
}

fn map_tag_summaries_error(error: &git2::Error, context: impl AsRef<str>) -> GitError {
    let error_code = if error.code() == ErrorCode::NotFound {
        GitErrorCode::RefNotFound
    } else {
        GitErrorCode::TagSummariesFailed
    };

    GitError::new(error_code, format!("{}: {error}", context.as_ref()))
}
