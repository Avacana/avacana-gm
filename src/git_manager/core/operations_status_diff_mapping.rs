use crate::git_manager::core::{DiffStatusCode, DiffStatusEntry};
use git2::Delta;
use std::collections::BTreeSet;

pub(super) const fn map_delta_to_diff_status_code(delta: Delta) -> Option<DiffStatusCode> {
    match delta {
        Delta::Added => Some(DiffStatusCode::Added),
        Delta::Deleted => Some(DiffStatusCode::Deleted),
        Delta::Modified => Some(DiffStatusCode::Modified),
        Delta::Renamed => Some(DiffStatusCode::Renamed),
        Delta::Copied => Some(DiffStatusCode::Copied),
        Delta::Typechange => Some(DiffStatusCode::TypeChange),
        Delta::Untracked => Some(DiffStatusCode::Untracked),
        Delta::Conflicted => Some(DiffStatusCode::Conflicted),
        Delta::Unmodified | Delta::Ignored | Delta::Unreadable => None,
    }
}

pub(super) fn delta_path(delta: &git2::DiffDelta<'_>) -> Option<String> {
    let raw_path = if matches!(delta.status(), Delta::Deleted) {
        delta.old_file().path().or_else(|| delta.new_file().path())
    } else {
        delta.new_file().path().or_else(|| delta.old_file().path())
    }?;
    Some(raw_path.to_string_lossy().into_owned())
}

pub(super) fn delta_identity_key(delta: &git2::DiffDelta<'_>) -> Option<String> {
    let path = delta_path(delta)?;
    let status_rank = diff_status_code_rank(map_delta_to_diff_status_code(delta.status())?);
    Some(format!("{status_rank}:{path}"))
}

pub(super) fn normalize_entries(entries: &mut Vec<DiffStatusEntry>) {
    entries.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(diff_status_code_rank(left.code).cmp(&diff_status_code_rank(right.code)))
    });
    let mut seen = BTreeSet::new();
    let mut next_unique = 0_usize;
    for index in 0..entries.len() {
        let key = (
            entries[index].path.clone(),
            diff_status_code_rank(entries[index].code),
        );
        if seen.insert(key) {
            entries.swap(next_unique, index);
            next_unique += 1;
        }
    }
    entries.truncate(next_unique);
}

pub(super) const fn diff_status_code_rank(code: DiffStatusCode) -> u8 {
    match code {
        DiffStatusCode::Added => 0,
        DiffStatusCode::Modified => 1,
        DiffStatusCode::Deleted => 2,
        DiffStatusCode::Renamed => 3,
        DiffStatusCode::Copied => 4,
        DiffStatusCode::TypeChange => 5,
        DiffStatusCode::Untracked => 6,
        DiffStatusCode::Conflicted => 7,
    }
}

pub(super) fn status_entry_path(entry: &git2::StatusEntry<'_>) -> String {
    entry.path().map_or_else(
        || String::from_utf8_lossy(entry.path_bytes()).into_owned(),
        str::to_owned,
    )
}

pub(super) fn count_unique_paths(entries: &[DiffStatusEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}
