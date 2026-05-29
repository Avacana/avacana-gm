use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, ReferenceDescriptor, ReferenceKind,
};
use git2::{ErrorCode, Oid, ReferenceType, Repository};

pub(crate) fn ensure_expected_target_matches(
    repository: &Repository,
    current_reference: Option<&git2::Reference<'_>>,
    expected_target: &str,
    reference_name: &str,
    field_name: &str,
) -> GitResult<()> {
    let expected_target = normalize_non_empty(expected_target, field_name)?;
    let Some(current_reference) = current_reference else {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!(
                "{field_name} expected `{expected_target}`, but reference `{reference_name}` does not exist"
            ),
        ));
    };

    if expected_target_matches_reference(repository, current_reference, expected_target) {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::RefNotFound,
        format!(
            "{field_name} mismatch for `{reference_name}`: expected `{expected_target}`, actual `{}`",
            describe_reference_target(current_reference)
        ),
    ))
}

pub(crate) fn normalize_notes_ref(namespace: Option<&str>) -> GitResult<Option<String>> {
    let Some(namespace) = namespace else {
        return Ok(None);
    };

    let namespace = normalize_non_empty(namespace, "refs.write_note.namespace")?;
    if namespace.starts_with("refs/notes/") {
        return Ok(Some(namespace.to_owned()));
    }

    Ok(Some(format!("refs/notes/{namespace}")))
}

pub(crate) fn reference_descriptor(reference: &git2::Reference<'_>) -> ReferenceDescriptor {
    let name = reference.name().map_or_else(
        || String::from_utf8_lossy(reference.name_bytes()).into_owned(),
        str::to_owned,
    );

    ReferenceDescriptor {
        name,
        reference_kind: reference.kind().map(|kind| match kind {
            ReferenceType::Direct => ReferenceKind::Direct,
            ReferenceType::Symbolic => ReferenceKind::Symbolic,
        }),
        target: reference.target().map(|oid| oid.to_string()),
        symbolic_target: reference.symbolic_target().map(str::to_owned),
    }
}

pub(crate) fn sort_reference_descriptors(descriptors: &mut [ReferenceDescriptor]) {
    descriptors.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.reference_kind.cmp(&right.reference_kind))
            .then(left.target.cmp(&right.target))
            .then(left.symbolic_target.cmp(&right.symbolic_target))
    });
}

pub(crate) fn normalize_reference_name<'a>(
    reference_name: &'a str,
    field_name: &str,
) -> GitResult<&'a str> {
    let reference_name = normalize_non_empty(reference_name, field_name)?;
    if reference_name != "HEAD" && !git2::Reference::is_valid_name(reference_name) {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} contains invalid git reference name `{reference_name}`"),
        ));
    }
    Ok(reference_name)
}

pub(crate) fn normalize_tag_name(tag_name: &str) -> GitResult<&str> {
    let normalized_tag_name = normalize_non_empty(tag_name, "refs.create_tag.name")?;
    let normalized_tag_name = normalized_tag_name
        .strip_prefix("refs/tags/")
        .unwrap_or(normalized_tag_name);
    normalize_non_empty(normalized_tag_name, "refs.create_tag.name")
}

pub(crate) fn normalize_non_empty<'a>(value: &'a str, field_name: &str) -> GitResult<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} must not be empty"),
        ));
    }
    if value.contains('\0') {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} must not contain NUL bytes"),
        ));
    }
    Ok(value)
}

pub(crate) fn normalize_optional_message<'a>(
    message: Option<&'a str>,
    field_name: &str,
) -> GitResult<Option<&'a str>> {
    let Some(message) = message else {
        return Ok(None);
    };

    let message = message.trim();
    if message.is_empty() {
        return Ok(None);
    }
    if message.contains('\0') {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} must not contain NUL bytes"),
        ));
    }

    Ok(Some(message))
}

pub(crate) fn map_refs_error(error: &git2::Error, context: impl AsRef<str>) -> GitError {
    let code = match error.code() {
        ErrorCode::NotFound
        | ErrorCode::UnbornBranch
        | ErrorCode::InvalidSpec
        | ErrorCode::Ambiguous
        | ErrorCode::Modified
        | ErrorCode::Invalid => GitErrorCode::RefNotFound,
        ErrorCode::Locked => GitErrorCode::LockContention,
        _ => GitErrorCode::Internal,
    };

    GitError::new(code, format!("{}: {error}", context.as_ref()))
}

fn expected_target_matches_reference(
    repository: &Repository,
    reference: &git2::Reference<'_>,
    expected_target: &str,
) -> bool {
    if let Some(symbolic_target) = reference.symbolic_target() {
        return symbolic_target == expected_target;
    }

    let Some(current_target) = reference.target() else {
        return false;
    };

    if let Ok(expected_oid) = Oid::from_str(expected_target) {
        return expected_oid == current_target;
    }

    repository
        .revparse_single(expected_target)
        .is_ok_and(|object| object.id() == current_target)
}

fn describe_reference_target(reference: &git2::Reference<'_>) -> String {
    if let Some(symbolic_target) = reference.symbolic_target() {
        return symbolic_target.to_owned();
    }
    if let Some(oid) = reference.target() {
        return oid.to_string();
    }
    "<none>".to_string()
}
