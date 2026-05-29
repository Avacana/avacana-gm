use crate::git_manager::core::operations_query_lifecycle_support::{
    collect_commit_change_entries, collect_revwalk_oids, commit_summary, empty_query_result,
    extract_message_details, map_query_error, normalize_optional_revision, resolve_commit_summary,
};
use crate::git_manager::core::{GitResult, QueryLifecycleResult, QueryShortlogEntry};
use git2::{Email, EmailCreateOptions, ObjectType, Repository};
use std::collections::BTreeMap;

pub(super) fn execute_log_operation(
    repository: &Repository,
    revision_range: Option<&str>,
    max_count: usize,
) -> GitResult<QueryLifecycleResult> {
    let commit_oids = collect_revwalk_oids(repository, revision_range, max_count, "log")?;
    let mut result = empty_query_result();
    result.commits = commit_oids
        .iter()
        .map(|oid| resolve_commit_summary(repository, *oid, "query_lifecycle.log"))
        .collect::<GitResult<Vec<_>>>()?;
    Ok(result)
}

pub(super) fn execute_shortlog_operation(
    repository: &Repository,
    revision_range: Option<&str>,
    max_count: usize,
) -> GitResult<QueryLifecycleResult> {
    let commit_oids = collect_revwalk_oids(repository, revision_range, max_count, "shortlog")?;
    let commit_summaries = commit_oids
        .iter()
        .map(|oid| resolve_commit_summary(repository, *oid, "query_lifecycle.shortlog"))
        .collect::<GitResult<Vec<_>>>()?;
    let mut shortlog_aggregation = BTreeMap::<(String, String), usize>::new();
    for summary in commit_summaries {
        let author_name = summary.author_name.unwrap_or_else(|| "unknown".to_string());
        let author_email = summary
            .author_email
            .unwrap_or_else(|| "unknown@unknown.invalid".to_string());
        *shortlog_aggregation
            .entry((author_name, author_email))
            .or_insert(0) += 1;
    }
    let mut shortlog_entries = shortlog_aggregation
        .into_iter()
        .map(
            |((author_name, author_email), commit_count)| QueryShortlogEntry {
                author_name,
                author_email,
                commit_count,
            },
        )
        .collect::<Vec<_>>();
    shortlog_entries.sort_by(|left, right| {
        right
            .commit_count
            .cmp(&left.commit_count)
            .then_with(|| left.author_name.cmp(&right.author_name))
            .then_with(|| left.author_email.cmp(&right.author_email))
    });
    let mut result = empty_query_result();
    result.shortlog_entries = shortlog_entries;
    Ok(result)
}

pub(super) fn execute_show_operation(
    repository: &Repository,
    revision: Option<&str>,
) -> GitResult<QueryLifecycleResult> {
    let revision = normalize_optional_revision(revision).unwrap_or("HEAD");
    let object: git2::Object<'_> = repository.revparse_single(revision).map_err(|error| {
        map_query_error(
            &error,
            format!("query_lifecycle.show failed to resolve `{revision}`"),
        )
    })?;
    let object_kind = object
        .kind()
        .map_or_else(|| "unknown".to_string(), |kind| kind.str().to_string());
    let mut result = empty_query_result();
    result.summary = Some(format!(
        "show `{revision}` -> {object_kind} {}",
        object.id()
    ));
    match object.kind() {
        Some(ObjectType::Commit) => {
            let commit = object.peel_to_commit().map_err(|error| {
                map_query_error(
                    &error,
                    "query_lifecycle.show failed to peel object to commit",
                )
            })?;
            result.commits.push(commit_summary(&commit));
            result.change_entries = collect_commit_change_entries(repository, &commit)?;
        }
        Some(ObjectType::Tree) => {
            let tree = object.peel_to_tree().map_err(|error| {
                map_query_error(&error, "query_lifecycle.show failed to peel object to tree")
            })?;
            crate::git_manager::core::operations_query_lifecycle_support::collect_tree_entries(
                repository,
                &tree,
                "",
                false,
                &mut result.tree_entries,
                "query_lifecycle.show",
            )?;
        }
        Some(ObjectType::Tag) => {
            if let Ok(commit) = object.peel_to_commit() {
                result.commits.push(commit_summary(&commit));
                result.change_entries = collect_commit_change_entries(repository, &commit)?;
            } else if let Ok(tree) = object.peel_to_tree() {
                crate::git_manager::core::operations_query_lifecycle_support::collect_tree_entries(
                    repository,
                    &tree,
                    "",
                    false,
                    &mut result.tree_entries,
                    "query_lifecycle.show",
                )?;
            }
        }
        _ => {}
    }
    Ok(result)
}

pub(super) fn execute_message_trailers_operation(
    repository: &Repository,
    revision: Option<&str>,
) -> GitResult<QueryLifecycleResult> {
    let revision = normalize_optional_revision(revision).unwrap_or("HEAD");
    let commit = resolve_revision_commit(repository, revision, "query_lifecycle.message_trailers")?;
    let mut result = empty_query_result();
    result.message_details = Some(extract_message_details(
        commit.message_raw().unwrap_or_default(),
        "query_lifecycle.message_trailers",
    )?);
    result.summary = Some(format!("parsed message trailers for `{revision}`"));
    Ok(result)
}

pub(super) fn execute_format_email_operation(
    repository: &Repository,
    revision: Option<&str>,
    subject_prefix: Option<&str>,
) -> GitResult<QueryLifecycleResult> {
    let revision = normalize_optional_revision(revision).unwrap_or("HEAD");
    let commit = resolve_revision_commit(repository, revision, "query_lifecycle.format_email")?;
    let mut email_options = EmailCreateOptions::new();
    email_options.always_number(true).start_number(1);
    if let Some(subject_prefix) = subject_prefix {
        let subject_prefix =
            crate::git_manager::core::operations_query_lifecycle_support::normalize_non_empty(
                subject_prefix,
                "query_lifecycle.format_email.subject_prefix",
            )?;
        email_options.subject_prefix(subject_prefix);
    }
    let email = Email::from_commit(&commit, &mut email_options).map_err(|error| {
        map_query_error(
            &error,
            format!("query_lifecycle.format_email failed to format `{revision}`"),
        )
    })?;
    let mut result = empty_query_result();
    result.formatted_email = Some(String::from_utf8_lossy(email.as_slice()).into_owned());
    result.summary = Some(format!("formatted email patch for `{revision}`"));
    Ok(result)
}

pub(super) fn resolve_revision_commit<'repo>(
    repository: &'repo Repository,
    revision: &str,
    operation: &str,
) -> GitResult<git2::Commit<'repo>> {
    repository
        .revparse_single(revision)
        .map_err(|error| {
            map_query_error(
                &error,
                format!("{operation} failed to resolve `{revision}`"),
            )
        })?
        .peel_to_commit()
        .map_err(|error| {
            map_query_error(
                &error,
                format!("{operation} failed to peel `{revision}` to commit"),
            )
        })
}

pub(super) fn execute_whatchanged_operation(
    repository: &Repository,
    revision_range: Option<&str>,
    max_count: usize,
) -> GitResult<QueryLifecycleResult> {
    let commit_oids = collect_revwalk_oids(repository, revision_range, max_count, "whatchanged")?;
    let mut commits = Vec::with_capacity(commit_oids.len());
    let mut change_entries = Vec::new();
    for oid in commit_oids {
        let commit = repository.find_commit(oid).map_err(|error| {
            map_query_error(
                &error,
                format!("query_lifecycle.whatchanged failed to load commit `{oid}`"),
            )
        })?;
        commits.push(commit_summary(&commit));
        change_entries.extend(collect_commit_change_entries(repository, &commit)?);
    }
    let mut result = empty_query_result();
    result.commits = commits;
    result.change_entries = change_entries;
    Ok(result)
}
