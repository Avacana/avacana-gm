use crate::git_manager::core::operations_refs_meta_support::{
    collect_branch_descriptors, collect_config_entries, collect_reference_descriptors,
    collect_reference_names,
};
use crate::git_manager::core::{GitResult, RefsResult};
use git2::Repository;

pub(super) fn execute_list_operation(
    repository: &Repository,
    pattern: Option<&str>,
) -> GitResult<RefsResult> {
    let references = collect_reference_descriptors(repository, pattern)?;
    Ok(RefsResult {
        changed: false,
        references,
        ..RefsResult::default()
    })
}

pub(super) fn execute_list_branches_operation(
    repository: &Repository,
    include_local: bool,
    include_remote: bool,
) -> GitResult<RefsResult> {
    let branches = collect_branch_descriptors(repository, include_local, include_remote)?;
    Ok(RefsResult {
        changed: false,
        branches,
        ..RefsResult::default()
    })
}

pub(super) fn execute_list_reference_names_operation(
    repository: &Repository,
    pattern: Option<&str>,
) -> GitResult<RefsResult> {
    let reference_names = collect_reference_names(repository, pattern)?;
    Ok(RefsResult {
        changed: false,
        reference_names,
        ..RefsResult::default()
    })
}

pub(super) fn execute_list_config_entries_operation(
    repository: &Repository,
    glob: Option<&str>,
) -> GitResult<RefsResult> {
    let config_entries = collect_config_entries(repository, glob)?;
    Ok(RefsResult {
        changed: false,
        config_entries,
        ..RefsResult::default()
    })
}
