use crate::git_manager::core::operations_query_lifecycle_support::{
    collect_tree_entries, collect_tree_entries_walk, empty_query_result,
    normalize_optional_revision, resolve_revspec, resolve_tree,
};
use crate::git_manager::core::{GitResult, QueryLifecycleResult};
use git2::{Repository, TreeWalkMode};

use super::DEFAULT_TREE_REVISION;

pub(super) fn execute_revparse_operation(
    repository: &Repository,
    spec: &str,
) -> GitResult<QueryLifecycleResult> {
    let mut result = empty_query_result();
    result.revspec = Some(resolve_revspec(repository, spec)?);
    Ok(result)
}

pub(super) fn execute_ls_tree_operation(
    repository: &Repository,
    revision: Option<&str>,
    recursive: bool,
) -> GitResult<QueryLifecycleResult> {
    let revision = normalize_optional_revision(revision).unwrap_or(DEFAULT_TREE_REVISION);
    let tree = resolve_tree(repository, revision, "query_lifecycle.ls_tree.revision")?;
    let mut result = empty_query_result();
    collect_tree_entries(
        repository,
        &tree,
        "",
        recursive,
        &mut result.tree_entries,
        "query_lifecycle.ls_tree",
    )?;
    result.summary = Some(format!(
        "listed {} entries from `{revision}`",
        result.tree_entries.len()
    ));
    Ok(result)
}

pub(super) fn execute_tree_walk_operation(
    repository: &Repository,
    revision: Option<&str>,
    post_order: bool,
) -> GitResult<QueryLifecycleResult> {
    let revision = normalize_optional_revision(revision).unwrap_or(DEFAULT_TREE_REVISION);
    let tree = resolve_tree(repository, revision, "query_lifecycle.tree_walk.revision")?;
    let walk_mode = if post_order {
        TreeWalkMode::PostOrder
    } else {
        TreeWalkMode::PreOrder
    };
    let mut result = empty_query_result();
    collect_tree_entries_walk(
        &tree,
        "",
        walk_mode,
        &mut result.tree_entries,
        "query_lifecycle.tree_walk",
    )?;
    result.summary = Some(format!(
        "tree walk ({}) from `{revision}` produced {} entries",
        if post_order {
            "post-order"
        } else {
            "pre-order"
        },
        result.tree_entries.len()
    ));
    Ok(result)
}
