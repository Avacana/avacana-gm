use super::{map_refs_error, normalize_non_empty};
use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{ErrorCode, Oid, Repository};

#[derive(Debug, Clone)]
pub(crate) enum ResolvedReferenceTarget {
    Direct(Oid),
    Symbolic(String),
}

pub(crate) fn find_reference<'repo>(
    repository: &'repo Repository,
    reference_name: &str,
    operation: &str,
) -> GitResult<git2::Reference<'repo>> {
    repository
        .find_reference(reference_name)
        .map_err(|error| map_reference_lookup_error(&error, operation, reference_name))
}

pub(crate) fn find_reference_optional<'repo>(
    repository: &'repo Repository,
    reference_name: &str,
    operation: &str,
) -> GitResult<Option<git2::Reference<'repo>>> {
    match repository.find_reference(reference_name) {
        Ok(reference) => Ok(Some(reference)),
        Err(error) if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) => {
            Ok(None)
        }
        Err(error) => Err(map_reference_lookup_error(
            &error,
            operation,
            reference_name,
        )),
    }
}

pub(crate) fn apply_reference_update<'repo>(
    repository: &'repo Repository,
    reference: &mut git2::Reference<'repo>,
    reference_name: &str,
    new_target: &ResolvedReferenceTarget,
    reflog_message: &str,
) -> GitResult<git2::Reference<'repo>> {
    match new_target {
        ResolvedReferenceTarget::Direct(oid) => {
            if reference.target().is_some() {
                reference.set_target(*oid, reflog_message)
            } else {
                repository.reference(reference_name, *oid, true, reflog_message)
            }
        }
        ResolvedReferenceTarget::Symbolic(target) => {
            if reference.symbolic_target().is_some() {
                reference.symbolic_set_target(target.as_str(), reflog_message)
            } else {
                repository.reference_symbolic(reference_name, target.as_str(), true, reflog_message)
            }
        }
    }
    .map_err(|error| {
        map_refs_error(
            &error,
            format!("refs.update_reference failed to update `{reference_name}`"),
        )
    })
}

pub(crate) fn create_reference_with_target<'repo>(
    repository: &'repo Repository,
    reference_name: &str,
    new_target: &ResolvedReferenceTarget,
    reflog_message: &str,
) -> GitResult<git2::Reference<'repo>> {
    match new_target {
        ResolvedReferenceTarget::Direct(oid) => {
            repository.reference(reference_name, *oid, true, reflog_message)
        }
        ResolvedReferenceTarget::Symbolic(target) => {
            repository.reference_symbolic(reference_name, target.as_str(), true, reflog_message)
        }
    }
    .map_err(|error| {
        map_refs_error(
            &error,
            format!("refs.update_reference failed to create `{reference_name}`"),
        )
    })
}

pub(crate) fn resolve_reference_target(
    repository: &Repository,
    target_spec: &str,
    field_name: &str,
) -> GitResult<ResolvedReferenceTarget> {
    let target_spec = normalize_non_empty(target_spec, field_name)?;
    if looks_like_symbolic_target(target_spec) {
        validate_symbolic_target_name(target_spec, field_name)?;
        return Ok(ResolvedReferenceTarget::Symbolic(target_spec.to_owned()));
    }

    if let Ok(oid) = Oid::from_str(target_spec) {
        return Ok(ResolvedReferenceTarget::Direct(oid));
    }

    repository
        .revparse_single(target_spec)
        .map(|object| ResolvedReferenceTarget::Direct(object.id()))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!(
                    "{field_name} value `{target_spec}` cannot be resolved to object id: {error}"
                ),
            )
        })
}

pub(crate) fn resolve_target_oid(
    repository: &Repository,
    target_spec: &str,
    field_name: &str,
) -> GitResult<Oid> {
    repository
        .revparse_single(target_spec)
        .map(|object| object.id())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!(
                    "{field_name} value `{target_spec}` cannot be resolved to object id: {error}"
                ),
            )
        })
}

pub(crate) fn resolve_notes_reference_name(
    repository: &Repository,
    notes_ref: Option<&str>,
) -> GitResult<String> {
    if let Some(notes_ref) = notes_ref {
        return Ok(notes_ref.to_owned());
    }

    repository.note_default_ref().map_err(|error| {
        map_refs_error(
            &error,
            "refs.write_note failed to resolve default notes namespace",
        )
    })
}

pub(crate) fn target_matches_reference(
    reference: &git2::Reference<'_>,
    target: &ResolvedReferenceTarget,
) -> bool {
    match target {
        ResolvedReferenceTarget::Direct(oid) => reference
            .target()
            .is_some_and(|current_oid| current_oid == *oid),
        ResolvedReferenceTarget::Symbolic(symbolic_target) => reference
            .symbolic_target()
            .is_some_and(|current_symbolic_target| {
                current_symbolic_target == symbolic_target.as_str()
            }),
    }
}

fn looks_like_symbolic_target(target_spec: &str) -> bool {
    target_spec == "HEAD" || target_spec.starts_with("refs/")
}

fn validate_symbolic_target_name(target_spec: &str, field_name: &str) -> GitResult<()> {
    if target_spec == "HEAD" || git2::Reference::is_valid_name(target_spec) {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::RefNotFound,
        format!("{field_name} symbolic target `{target_spec}` is not a valid reference name"),
    ))
}

fn map_reference_lookup_error(
    error: &git2::Error,
    operation: &str,
    reference_name: &str,
) -> GitError {
    let code = if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) {
        GitErrorCode::RefNotFound
    } else {
        GitErrorCode::Internal
    };

    GitError::new(
        code,
        format!("operation `{operation}` failed to resolve reference `{reference_name}`: {error}"),
    )
}
