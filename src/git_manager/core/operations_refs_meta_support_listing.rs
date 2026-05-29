use super::{
    map_refs_error, normalize_non_empty, reference_descriptor, sort_reference_descriptors,
};
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, ReferenceDescriptor};
use git2::{
    Branch, BranchType, Branches, ConfigEntries, ConfigEntry, ConfigLevel, References, Repository,
};

pub(crate) fn collect_reference_descriptors(
    repository: &Repository,
    pattern: Option<&str>,
) -> GitResult<Vec<ReferenceDescriptor>> {
    let mut references: References<'_> = if let Some(pattern) = pattern {
        let pattern = normalize_non_empty(pattern, "refs.list.pattern")?;
        repository.references_glob(pattern).map_err(|error| {
            map_refs_error(
                &error,
                format!("refs.list failed to open reference iterator for glob `{pattern}`"),
            )
        })?
    } else {
        repository.references().map_err(|error| {
            map_refs_error(&error, "refs.list failed to open full reference iterator")
        })?
    };

    let mut descriptors = Vec::new();
    for reference in &mut references {
        let reference = reference.map_err(|error| {
            map_refs_error(&error, "refs.list failed while iterating reference entries")
        })?;
        descriptors.push(reference_descriptor(&reference));
    }

    sort_reference_descriptors(&mut descriptors);
    Ok(descriptors)
}

pub(crate) fn collect_branch_descriptors(
    repository: &Repository,
    include_local: bool,
    include_remote: bool,
) -> GitResult<Vec<ReferenceDescriptor>> {
    let branch_filter = match (include_local, include_remote) {
        (true, true) => None,
        (true, false) => Some(BranchType::Local),
        (false, true) => Some(BranchType::Remote),
        (false, false) => {
            return Err(GitError::new(
                GitErrorCode::RefNotFound,
                "refs.list_branches requires include_local=true and/or include_remote=true",
            ));
        }
    };

    let mut branches: Branches<'_> = repository.branches(branch_filter).map_err(|error| {
        map_refs_error(&error, "refs.list_branches failed to open branch iterator")
    })?;

    let mut descriptors = Vec::new();
    for branch_entry in &mut branches {
        let (branch, branch_type): (Branch<'_>, BranchType) = branch_entry.map_err(|error| {
            map_refs_error(
                &error,
                "refs.list_branches failed while iterating branch entries",
            )
        })?;

        let descriptor = match branch_type {
            BranchType::Local | BranchType::Remote => reference_descriptor(branch.get()),
        };
        descriptors.push(descriptor);
    }

    sort_reference_descriptors(&mut descriptors);
    Ok(descriptors)
}

pub(crate) fn collect_reference_names(
    repository: &Repository,
    pattern: Option<&str>,
) -> GitResult<Vec<String>> {
    let mut references: References<'_> = if let Some(pattern) = pattern {
        let pattern = normalize_non_empty(pattern, "refs.list_reference_names.pattern")?;
        repository.references_glob(pattern).map_err(|error| {
            map_refs_error(
                &error,
                format!(
                    "refs.list_reference_names failed to open reference names iterator for glob `{pattern}`"
                ),
            )
        })?
    } else {
        repository.references().map_err(|error| {
            map_refs_error(
                &error,
                "refs.list_reference_names failed to open full reference names iterator",
            )
        })?
    };

    let mut reference_names = references.names();
    let mut names = Vec::new();
    for name in &mut reference_names {
        let name = name.map_err(|error| {
            map_refs_error(
                &error,
                "refs.list_reference_names failed while iterating reference names",
            )
        })?;
        names.push(name.to_owned());
    }

    names.sort_unstable();
    names.dedup();
    Ok(names)
}

pub(crate) fn collect_config_entries(
    repository: &Repository,
    glob: Option<&str>,
) -> GitResult<Vec<(String, Option<String>, String)>> {
    let config = repository.config().map_err(|error| {
        map_refs_error(
            &error,
            "refs.list_config_entries failed to open repository config",
        )
    })?;

    let normalized_glob = glob
        .map(|value| normalize_non_empty(value, "refs.list_config_entries.glob"))
        .transpose()?;
    let mut entries: ConfigEntries<'_> = config.entries(normalized_glob).map_err(|error| {
        normalized_glob.map_or_else(
            || {
                map_refs_error(
                    &error,
                    "refs.list_config_entries failed to open full config iterator",
                )
            },
            |glob| {
                map_refs_error(
                    &error,
                    format!(
                        "refs.list_config_entries failed to open config iterator for glob `{glob}`"
                    ),
                )
            },
        )
    })?;

    let mut descriptors = Vec::new();
    while let Some(entry) = entries.next() {
        let entry: &ConfigEntry<'_> = entry.map_err(|error| {
            map_refs_error(
                &error,
                "refs.list_config_entries failed while iterating config entries",
            )
        })?;
        descriptors.push((
            config_entry_name(entry),
            config_entry_value(entry),
            config_level_label(entry.level()).to_owned(),
        ));
    }

    descriptors.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(left.1.cmp(&right.1))
            .then(left.2.cmp(&right.2))
    });
    Ok(descriptors)
}

fn config_entry_name(entry: &ConfigEntry<'_>) -> String {
    entry.name().map_or_else(
        || String::from_utf8_lossy(entry.name_bytes()).into_owned(),
        str::to_owned,
    )
}

fn config_entry_value(entry: &ConfigEntry<'_>) -> Option<String> {
    if !entry.has_value() {
        return None;
    }

    Some(entry.value().map_or_else(
        || String::from_utf8_lossy(entry.value_bytes()).into_owned(),
        str::to_owned,
    ))
}

const fn config_level_label(level: ConfigLevel) -> &'static str {
    match level {
        ConfigLevel::ProgramData => "programdata",
        ConfigLevel::System => "system",
        ConfigLevel::XDG => "xdg",
        ConfigLevel::Global => "global",
        ConfigLevel::Local => "local",
        ConfigLevel::Worktree => "worktree",
        ConfigLevel::App => "app",
        ConfigLevel::Highest => "highest",
    }
}
