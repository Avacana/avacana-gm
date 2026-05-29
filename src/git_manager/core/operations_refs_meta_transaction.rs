use crate::git_manager::core::operations_refs_meta_support::{
    ensure_expected_target_matches, find_reference_optional, map_refs_error,
    normalize_optional_message, normalize_reference_name, reference_descriptor,
    resolve_reference_target, sort_reference_descriptors, target_matches_reference,
    ResolvedReferenceTarget,
};
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, RefUpdateSpec, RefsResult};
use git2::Repository;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
struct PreparedTransactionUpdate {
    reference_name: String,
    new_target: ResolvedReferenceTarget,
}

pub(super) fn execute_transaction_operation(
    repository: &Repository,
    updates: &[RefUpdateSpec],
    reflog_message: Option<&str>,
) -> GitResult<RefsResult> {
    let prepared_updates = prepare_transaction_updates(repository, updates)?;
    if prepared_updates.is_empty() {
        return Ok(RefsResult {
            changed: false,
            ..RefsResult::default()
        });
    }
    let reflog_message =
        normalize_optional_message(reflog_message, "refs.transaction.reflog_message")?
            .unwrap_or("git_manager refs transaction");
    let mut transaction = repository.transaction().map_err(|error| {
        map_refs_error(
            &error,
            "refs.transaction failed to initialize transaction handle",
        )
    })?;
    for update in &prepared_updates {
        transaction
            .lock_ref(update.reference_name.as_str())
            .map_err(|error| {
                map_refs_error(
                    &error,
                    format!(
                        "refs.transaction failed to lock reference `{}`",
                        update.reference_name
                    ),
                )
            })?;
    }
    for update in &prepared_updates {
        match &update.new_target {
            ResolvedReferenceTarget::Direct(oid) => {
                transaction.set_target(update.reference_name.as_str(), *oid, None, reflog_message)
            }
            ResolvedReferenceTarget::Symbolic(target) => transaction.set_symbolic_target(
                update.reference_name.as_str(),
                target.as_str(),
                None,
                reflog_message,
            ),
        }
        .map_err(|error| {
            map_refs_error(
                &error,
                format!(
                    "refs.transaction failed to stage update for `{}`",
                    update.reference_name
                ),
            )
        })?;
    }
    transaction.commit().map_err(|error| {
        map_refs_error(
            &error,
            "refs.transaction failed to commit staged reference updates",
        )
    })?;
    let mut references = Vec::with_capacity(prepared_updates.len());
    for update in prepared_updates {
        if let Some(reference) = find_reference_optional(
            repository,
            update.reference_name.as_str(),
            "refs.transaction.result",
        )? {
            references.push(reference_descriptor(&reference));
        }
    }
    sort_reference_descriptors(&mut references);
    Ok(RefsResult {
        changed: true,
        references,
        ..RefsResult::default()
    })
}

fn prepare_transaction_updates(
    repository: &Repository,
    updates: &[RefUpdateSpec],
) -> GitResult<Vec<PreparedTransactionUpdate>> {
    if updates.is_empty() {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            "refs.transaction requires at least one update entry",
        ));
    }
    let mut seen_reference_names = BTreeSet::new();
    let mut prepared_updates = Vec::with_capacity(updates.len());
    for update in updates {
        let reference_name = normalize_reference_name(
            update.reference_name.as_str(),
            "refs.transaction.reference_name",
        )?
        .to_owned();
        if !seen_reference_names.insert(reference_name.clone()) {
            return Err(GitError::new(
                GitErrorCode::RefNotFound,
                format!("refs.transaction includes duplicate update for `{reference_name}`"),
            ));
        }
        let current_reference = find_reference_optional(
            repository,
            reference_name.as_str(),
            "refs.transaction.preflight.lookup",
        )?;
        if let Some(expected_old_target) = update.expected_old_target.as_deref() {
            ensure_expected_target_matches(
                repository,
                current_reference.as_ref(),
                expected_old_target,
                reference_name.as_str(),
                "refs.transaction.expected_old_target",
            )?;
        }
        let new_target = resolve_reference_target(
            repository,
            update.new_target.as_str(),
            "refs.transaction.new_target",
        )?;
        if let Some(current_reference) = current_reference.as_ref() {
            if target_matches_reference(current_reference, &new_target) {
                continue;
            }
        }
        prepared_updates.push(PreparedTransactionUpdate {
            reference_name,
            new_target,
        });
    }
    Ok(prepared_updates)
}
