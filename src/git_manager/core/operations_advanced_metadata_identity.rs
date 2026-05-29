use crate::git_manager::core::operations_advanced_support::{
    empty_advanced_result, map_advanced_error, normalize_optional_non_empty,
};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::{DescribeFormatOptions, DescribeOptions, Repository};

const DEFAULT_DESCRIBE_REVISION: &str = "HEAD";
const DEFAULT_MAILMAP_NAME: &str = "unknown";
const DEFAULT_MAILMAP_EMAIL: &str = "unknown@unknown.invalid";

pub(crate) fn execute_resolve_mailmap_operation(
    repository: &Repository,
    name: Option<&str>,
    email: Option<&str>,
) -> GitResult<AdvancedResult> {
    let name = normalize_optional_non_empty(name, "advanced.resolve_mailmap.name")?;
    let email = normalize_optional_non_empty(email, "advanced.resolve_mailmap.email")?;

    if name.is_none() && email.is_none() {
        return Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            "advanced.resolve_mailmap requires at least one of `name` or `email`",
        ));
    }

    let signature_name = name.unwrap_or(DEFAULT_MAILMAP_NAME);
    let signature_email = email.unwrap_or(DEFAULT_MAILMAP_EMAIL);
    let input_signature =
        git2::Signature::now(signature_name, signature_email).map_err(|error| {
            GitError::new(
                GitErrorCode::AdvancedInvalidInput,
                format!("advanced.resolve_mailmap failed to build input signature: {error}"),
            )
        })?;

    let mailmap = repository.mailmap().map_err(|error| {
        map_advanced_error(
            &error,
            "advanced.resolve_mailmap failed to load repository mailmap",
        )
    })?;
    let resolved_signature = mailmap
        .resolve_signature(&input_signature)
        .map_err(|error| {
            map_advanced_error(
                &error,
                "advanced.resolve_mailmap failed to resolve identity",
            )
        })?;

    let resolved_name = resolved_signature
        .name()
        .map_or_else(|| DEFAULT_MAILMAP_NAME.to_string(), str::to_owned);
    let resolved_email = resolved_signature
        .email()
        .map_or_else(|| DEFAULT_MAILMAP_EMAIL.to_string(), str::to_owned);

    let mut result = empty_advanced_result();
    result.items.push(format!("resolved_name:{resolved_name}"));
    result
        .items
        .push(format!("resolved_email:{resolved_email}"));
    if let Some(name) = name {
        result.items.push(format!("input_name:{name}"));
    }
    if let Some(email) = email {
        result.items.push(format!("input_email:{email}"));
    }
    result.summary = Some(format!(
        "mailmap resolved signature to `{resolved_name} <{resolved_email}>`"
    ));

    Ok(result)
}

pub(crate) fn execute_describe_revision_operation(
    repository: &Repository,
    revision: Option<&str>,
) -> GitResult<AdvancedResult> {
    let revision = normalize_optional_non_empty(revision, "advanced.describe_revision.revision")?
        .unwrap_or(DEFAULT_DESCRIBE_REVISION);

    let object = repository.revparse_single(revision).map_err(|error| {
        map_advanced_error(
            &error,
            format!("advanced.describe_revision failed to resolve `{revision}`"),
        )
    })?;

    let mut describe_options = DescribeOptions::new();
    describe_options
        .describe_tags()
        .show_commit_oid_as_fallback(true)
        .max_candidates_tags(64);

    let description = object
        .describe(&describe_options)
        .and_then(|describe| {
            let mut format_options = DescribeFormatOptions::new();
            format_options.abbreviated_size(12).dirty_suffix("-dirty");
            describe.format(Some(&format_options))
        })
        .map_err(|error| {
            map_advanced_error(
                &error,
                format!("advanced.describe_revision failed to describe `{revision}`"),
            )
        })?;

    let mut result = empty_advanced_result();
    result.summary = Some(description.clone());
    result.items.push(format!("revision:{revision}"));
    result.items.push(format!("description:{description}"));
    result.items.push(description);
    Ok(result)
}
