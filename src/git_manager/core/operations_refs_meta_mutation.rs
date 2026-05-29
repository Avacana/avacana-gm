use crate::git_manager::core::operations_branch::execute_create_branch_operation;
use crate::git_manager::core::operations_refs_meta_support::{
    apply_reference_update, create_reference_with_target, ensure_expected_target_matches,
    find_reference, find_reference_descriptor, find_reference_optional, map_refs_error,
    normalize_non_empty, normalize_optional_message, normalize_reference_name, normalize_tag_name,
    reference_descriptor, resolve_reference_target, target_matches_reference,
};
use crate::git_manager::core::{
    CreateBranchRequest, GitError, GitErrorCode, GitResult, RefsResult,
};
use git2::Repository;
use std::path::Path;

pub(super) fn execute_create_branch_refs_operation(
    repository_path: &Path,
    repository: &Repository,
    branch_name: &str,
    start_point: Option<&str>,
) -> GitResult<RefsResult> {
    let create_result = execute_create_branch_operation(&CreateBranchRequest {
        repository_path: repository_path.to_path_buf(),
        branch_name: branch_name.to_owned(),
        start_point: start_point.map(str::to_owned),
    })?;
    let created_reference_name = format!("refs/heads/{}", create_result.branch_name);
    let descriptor = find_reference_descriptor(
        repository,
        created_reference_name.as_str(),
        "refs.create_branch",
    )?;
    Ok(RefsResult {
        changed: true,
        references: vec![descriptor],
        ..RefsResult::default()
    })
}

pub(super) fn execute_create_tag_operation(
    repository: &Repository,
    tag_name: &str,
    target_spec: &str,
    message: Option<&str>,
    force: bool,
) -> GitResult<RefsResult> {
    let tag_name = normalize_tag_name(tag_name)?;
    let target_spec = normalize_non_empty(target_spec, "refs.create_tag.target")?;
    let target_object = repository.revparse_single(target_spec).map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!(
                "refs.create_tag target `{target_spec}` cannot be resolved to a git object: {error}"
            ),
        )
    })?;
    if let Some(message) = message {
        let message = normalize_non_empty(message, "refs.create_tag.message")?;
        let signature = repository.signature().map_err(|error| {
            GitError::new(
                GitErrorCode::InvalidSignatureContext,
                format!("refs.create_tag failed to resolve signature for annotated tag: {error}"),
            )
        })?;
        repository
            .tag(tag_name, &target_object, &signature, message, force)
            .map_err(|error| {
                map_refs_error(
                    &error,
                    format!("refs.create_tag failed to create annotated tag `{tag_name}`"),
                )
            })?;
    } else {
        repository
            .tag_lightweight(tag_name, &target_object, force)
            .map_err(|error| {
                map_refs_error(
                    &error,
                    format!("refs.create_tag failed to create lightweight tag `{tag_name}`"),
                )
            })?;
    }
    let tag_reference_name = format!("refs/tags/{tag_name}");
    let descriptor =
        find_reference_descriptor(repository, tag_reference_name.as_str(), "refs.create_tag")?;
    Ok(RefsResult {
        changed: true,
        references: vec![descriptor],
        ..RefsResult::default()
    })
}

pub(super) fn execute_delete_reference_operation(
    repository: &Repository,
    reference_name: &str,
    expected_target: Option<&str>,
) -> GitResult<RefsResult> {
    let reference_name = normalize_reference_name(reference_name, "refs.delete_reference.name")?;
    let mut reference = find_reference(repository, reference_name, "refs.delete_reference")?;
    if let Some(expected_target) = expected_target {
        ensure_expected_target_matches(
            repository,
            Some(&reference),
            expected_target,
            reference_name,
            "refs.delete_reference.expected_target",
        )?;
    }
    let deleted_descriptor = reference_descriptor(&reference);
    reference.delete().map_err(|error| {
        map_refs_error(
            &error,
            format!("refs.delete_reference failed to delete `{reference_name}`"),
        )
    })?;
    Ok(RefsResult {
        changed: true,
        references: vec![deleted_descriptor],
        ..RefsResult::default()
    })
}

pub(super) fn execute_update_reference_operation(
    repository: &Repository,
    reference_name: &str,
    new_target: &str,
    expected_old_target: Option<&str>,
    reflog_message: Option<&str>,
) -> GitResult<RefsResult> {
    let reference_name = normalize_reference_name(reference_name, "refs.update_reference.name")?;
    let mut current_reference =
        find_reference_optional(repository, reference_name, "refs.update_reference")?;
    let resolved_target =
        resolve_reference_target(repository, new_target, "refs.update_reference.new_target")?;
    if let Some(reference) = current_reference.as_ref() {
        if let Some(expected_old_target) = expected_old_target {
            ensure_expected_target_matches(
                repository,
                Some(reference),
                expected_old_target,
                reference_name,
                "refs.update_reference.expected_old_target",
            )?;
        }
        if target_matches_reference(reference, &resolved_target) {
            return Ok(RefsResult {
                changed: false,
                references: vec![reference_descriptor(reference)],
                ..RefsResult::default()
            });
        }
    } else if let Some(expected_old_target) = expected_old_target {
        ensure_expected_target_matches(
            repository,
            None,
            expected_old_target,
            reference_name,
            "refs.update_reference.expected_old_target",
        )?;
    }
    let reflog_message =
        normalize_optional_message(reflog_message, "refs.update_reference.reflog_message")?
            .unwrap_or("git_manager refs update");
    let updated_reference = if let Some(reference) = current_reference.as_mut() {
        apply_reference_update(
            repository,
            reference,
            reference_name,
            &resolved_target,
            reflog_message,
        )?
    } else {
        create_reference_with_target(repository, reference_name, &resolved_target, reflog_message)?
    };
    Ok(RefsResult {
        changed: true,
        references: vec![reference_descriptor(&updated_reference)],
        ..RefsResult::default()
    })
}
