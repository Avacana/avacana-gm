use super::{
    is_absent_head_error, map_query_error, normalize_max_count, normalize_optional_revision,
};
use crate::git_manager::core::{GitResult, QueryCommitSummary};
use git2::{Oid, Repository, Sort};

pub(crate) fn collect_revwalk_oids(
    repository: &Repository,
    revision_range: Option<&str>,
    max_count: usize,
    operation: &str,
) -> GitResult<Vec<Oid>> {
    let max_count = normalize_max_count(max_count, operation)?;
    let mut revwalk: git2::Revwalk<'_> = repository.revwalk().map_err(|error| {
        map_query_error(
            &error,
            format!("query_lifecycle.{operation} failed to create revwalk"),
        )
    })?;

    revwalk
        .set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
        .map_err(|error| {
            map_query_error(
                &error,
                format!("query_lifecycle.{operation} failed to configure revwalk sorting"),
            )
        })?;

    match normalize_optional_revision(revision_range) {
        Some(revision_range) if revision_range.contains("..") => revwalk
            .push_range(revision_range)
            .map_err(|error| {
                map_query_error(
                    &error,
                    format!(
                        "query_lifecycle.{operation} failed to push revision range `{revision_range}`"
                    ),
                )
            })?,
        Some(revision_spec) => {
            let object = repository.revparse_single(revision_spec).map_err(|error| {
                map_query_error(
                    &error,
                    format!(
                        "query_lifecycle.{operation} failed to resolve revision `{revision_spec}`"
                    ),
                )
            })?;
            revwalk.push(object.id()).map_err(|error| {
                map_query_error(
                    &error,
                    format!(
                        "query_lifecycle.{operation} failed to push revision `{revision_spec}`"
                    ),
                )
            })?;
        }
        None => match revwalk.push_head() {
            Ok(()) => {}
            Err(error) if is_absent_head_error(&error) => return Ok(Vec::new()),
            Err(error) => {
                return Err(map_query_error(
                    &error,
                    format!("query_lifecycle.{operation} failed to push repository HEAD"),
                ));
            }
        },
    }

    let mut result = Vec::new();
    for oid in revwalk.take(max_count) {
        let oid = oid.map_err(|error| {
            map_query_error(
                &error,
                format!("query_lifecycle.{operation} failed to iterate revwalk"),
            )
        })?;
        result.push(oid);
    }

    Ok(result)
}

pub(crate) fn resolve_commit_summary(
    repository: &Repository,
    oid: Oid,
    operation: &str,
) -> GitResult<QueryCommitSummary> {
    let commit = repository.find_commit(oid).map_err(|error| {
        map_query_error(&error, format!("{operation} failed to load commit `{oid}`"))
    })?;
    Ok(commit_summary(&commit))
}

#[must_use]
pub(crate) fn commit_summary(commit: &git2::Commit<'_>) -> QueryCommitSummary {
    let author = commit.author();
    let summary = commit
        .summary()
        .map_or_else(|| commit.id().to_string(), str::to_owned);
    let parents: git2::Parents<'_, '_> = commit.parents();
    let parent_oids = parents.map(|parent| parent.id().to_string()).collect();

    QueryCommitSummary {
        oid: commit.id().to_string(),
        summary,
        author_name: author.name().map(str::to_owned),
        author_email: author.email().map(str::to_owned),
        timestamp_seconds: commit.time().seconds(),
        parent_count: commit.parent_count(),
        parent_oids,
    }
}
