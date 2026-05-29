use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{BranchType, Repository};

pub(super) fn resolve_target_reference_name(
    repository: &Repository,
    target_ref: Option<&str>,
    head_reference_name: &str,
) -> GitResult<String> {
    let Some(target_ref) = target_ref.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(head_reference_name.to_string());
    };

    if target_ref.starts_with("refs/") {
        return resolve_reference_name(repository, target_ref);
    }

    resolve_local_branch_reference_name(repository, target_ref)
}

fn resolve_reference_name(repository: &Repository, reference_name: &str) -> GitResult<String> {
    let reference = repository.find_reference(reference_name).map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("merge target reference `{reference_name}` not found: {error}"),
        )
    })?;

    reference
        .name()
        .and_then(non_empty)
        .map(str::to_owned)
        .ok_or_else(|| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!(
                    "merge target reference `{reference_name}` has empty canonical reference name"
                ),
            )
        })
}

fn resolve_local_branch_reference_name(
    repository: &Repository,
    branch_name: &str,
) -> GitResult<String> {
    let branch = repository
        .find_branch(branch_name, BranchType::Local)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("merge target local branch `{branch_name}` not found: {error}"),
            )
        })?;

    branch
        .get()
        .name()
        .and_then(non_empty)
        .map(str::to_owned)
        .ok_or_else(|| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("merge target local branch `{branch_name}` has an empty reference name"),
            )
        })
}

pub(super) fn resolve_source_annotated_commit<'repo>(
    repository: &'repo Repository,
    source_ref: &str,
) -> GitResult<git2::AnnotatedCommit<'repo>> {
    let source_object = repository.revparse_single(source_ref).map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("merge source `{source_ref}` cannot be resolved: {error}"),
        )
    })?;

    repository
        .find_annotated_commit(source_object.id())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!(
                    "merge source `{source_ref}` cannot be converted to annotated commit: {error}"
                ),
            )
        })
}

pub(super) fn resolve_attached_head_reference_name(repository: &Repository) -> GitResult<String> {
    let head_reference = repository.head().map_err(|error| {
        GitError::new(
            GitErrorCode::DetachedHead,
            format!("merge requires attached branch HEAD: {error}"),
        )
    })?;

    if !head_reference.is_branch() {
        return Err(GitError::new(
            GitErrorCode::DetachedHead,
            "merge requires attached branch HEAD",
        ));
    }

    head_reference
        .name()
        .and_then(non_empty)
        .map(str::to_owned)
        .ok_or_else(|| {
            GitError::new(
                GitErrorCode::DetachedHead,
                "merge failed to resolve canonical HEAD reference name",
            )
        })
}

pub(super) fn normalize_non_empty<'a>(value: &'a str, field_name: &str) -> GitResult<&'a str> {
    non_empty(value).ok_or_else(|| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} must not be empty"),
        )
    })
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}
