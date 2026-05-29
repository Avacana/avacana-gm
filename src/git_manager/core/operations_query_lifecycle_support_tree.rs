use super::map_query_error;
use crate::git_manager::core::{GitResult, QueryTreeEntry};
use git2::{Repository, TreeWalkMode, TreeWalkResult};

pub(crate) fn resolve_tree<'repo>(
    repository: &'repo Repository,
    revision: &str,
    operation: &str,
) -> GitResult<git2::Tree<'repo>> {
    repository
        .revparse_single(revision)
        .map_err(|error| {
            map_query_error(
                &error,
                format!("{operation} failed to resolve revision `{revision}`"),
            )
        })?
        .peel_to_tree()
        .map_err(|error| {
            map_query_error(
                &error,
                format!("{operation} failed to resolve tree for `{revision}`"),
            )
        })
}

pub(crate) fn collect_tree_entries(
    _repository: &Repository,
    tree: &git2::Tree<'_>,
    prefix: &str,
    recursive: bool,
    entries: &mut Vec<QueryTreeEntry>,
    operation: &str,
) -> GitResult<()> {
    if recursive {
        collect_tree_entries_walk(tree, prefix, TreeWalkMode::PreOrder, entries, operation)?;
        return Ok(());
    }

    let mut tree_iter: git2::TreeIter<'_> = tree.iter();
    for tree_entry in &mut tree_iter {
        push_tree_entry(prefix, &tree_entry, entries);
    }

    Ok(())
}

pub(crate) fn collect_tree_entries_walk(
    tree: &git2::Tree<'_>,
    prefix: &str,
    mode: TreeWalkMode,
    entries: &mut Vec<QueryTreeEntry>,
    operation: &str,
) -> GitResult<()> {
    tree.walk(mode, |root, tree_entry| {
        let normalized_root = root.trim_end_matches('/');
        let walk_prefix = if prefix.is_empty() {
            normalized_root.to_owned()
        } else if normalized_root.is_empty() {
            prefix.to_owned()
        } else {
            format!("{prefix}/{normalized_root}")
        };
        push_tree_entry(walk_prefix.as_str(), tree_entry, entries);
        TreeWalkResult::Ok
    })
    .map_err(|error| map_query_error(&error, format!("{operation} failed during tree walk")))?;
    Ok(())
}

fn push_tree_entry(
    prefix: &str,
    tree_entry: &git2::TreeEntry<'_>,
    entries: &mut Vec<QueryTreeEntry>,
) {
    let name = String::from_utf8_lossy(tree_entry.name_bytes()).into_owned();
    let path = if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    };
    let kind = tree_entry
        .kind()
        .map_or_else(|| "unknown".to_string(), |kind| kind.str().to_string());
    let file_mode = u32::try_from(tree_entry.filemode_raw()).map_or(0, |value| value);
    entries.push(QueryTreeEntry {
        path,
        object_id: tree_entry.id().to_string(),
        kind,
        file_mode,
    });
}
