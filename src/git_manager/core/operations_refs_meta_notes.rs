use crate::git_manager::core::operations_refs_meta_support::{
    find_reference_optional, map_refs_error, normalize_non_empty, normalize_notes_ref,
    reference_descriptor, resolve_notes_reference_name, resolve_target_oid,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, ReferenceDescriptor, ReferenceKind, RefsResult,
};
use git2::Repository;

pub(super) fn execute_write_note_operation(
    repository: &Repository,
    target_spec: &str,
    namespace: Option<&str>,
    message: &str,
    force: bool,
) -> GitResult<RefsResult> {
    let target_spec = normalize_non_empty(target_spec, "refs.write_note.target")?;
    let message = normalize_non_empty(message, "refs.write_note.message")?;
    let target_oid = resolve_target_oid(repository, target_spec, "refs.write_note.target")?;
    let notes_ref = normalize_notes_ref(namespace)?;
    let signature = repository.signature().map_err(|error| {
        GitError::new(
            GitErrorCode::InvalidSignatureContext,
            format!("refs.write_note failed to resolve signature: {error}"),
        )
    })?;
    let note_oid = repository
        .note(
            &signature,
            &signature,
            notes_ref.as_deref(),
            target_oid,
            message,
            force,
        )
        .map_err(|error| {
            map_refs_error(
                &error,
                format!("refs.write_note failed to write note for `{target_spec}`"),
            )
        })?;
    let notes_reference_name = resolve_notes_reference_name(repository, notes_ref.as_deref())?;
    let references = find_reference_optional(
        repository,
        notes_reference_name.as_str(),
        "refs.write_note.notes_ref",
    )?
    .map(|reference| vec![reference_descriptor(&reference)])
    .unwrap_or_default();
    Ok(RefsResult {
        changed: true,
        references,
        note_oid: Some(note_oid.to_string()),
        ..RefsResult::default()
    })
}

pub(super) fn execute_read_note_operation(
    repository: &Repository,
    target_spec: &str,
    namespace: Option<&str>,
) -> GitResult<RefsResult> {
    let target_spec = normalize_non_empty(target_spec, "refs.read_note.target")?;
    let target_oid = resolve_target_oid(repository, target_spec, "refs.read_note.target")?;
    let notes_ref = normalize_notes_ref(namespace)?;
    let notes_reference_name = resolve_notes_reference_name(repository, notes_ref.as_deref())?;
    let note: git2::Note<'_> = repository
        .find_note(notes_ref.as_deref(), target_oid)
        .map_err(|error| {
            map_refs_error(
                &error,
                format!("refs.read_note failed to resolve note for `{target_spec}`"),
            )
        })?;
    let note_oid = note.id();
    let notes_iter: git2::Notes<'_> = repository.notes(notes_ref.as_deref()).map_err(|error| {
        map_refs_error(
            &error,
            format!("refs.read_note failed to open notes iterator for `{target_spec}`"),
        )
    })?;
    let mut has_note_in_iterator = false;
    for note_entry in notes_iter {
        let (iter_note_oid, annotated_oid): (git2::Oid, git2::Oid) =
            note_entry.map_err(|error| {
                map_refs_error(
                    &error,
                    format!("refs.read_note failed while iterating notes for `{target_spec}`"),
                )
            })?;
        if annotated_oid == target_oid && iter_note_oid == note_oid {
            has_note_in_iterator = true;
            break;
        }
    }
    if !has_note_in_iterator {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!("refs.read_note note for `{target_spec}` not found in notes iterator"),
        ));
    }
    let references = vec![ReferenceDescriptor {
        name: format!("{notes_reference_name}/{target_oid}"),
        reference_kind: Some(ReferenceKind::Direct),
        target: Some(note_oid.to_string()),
        symbolic_target: None,
    }];
    Ok(RefsResult {
        changed: false,
        references,
        note_oid: Some(note_oid.to_string()),
        ..RefsResult::default()
    })
}
