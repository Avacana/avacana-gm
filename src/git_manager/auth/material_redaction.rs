pub(super) fn normalize_lookup_path(path: &str) -> Option<String> {
    let normalized = path.trim().trim_start_matches('/').to_string();
    (!normalized.is_empty()).then_some(normalized)
}

pub(super) fn push_unique(candidates: &mut Vec<String>, candidate: String) {
    if candidates.iter().any(|current| current == &candidate) {
        return;
    }
    candidates.push(candidate);
}

pub(super) fn redact_identity(identity: &str) -> String {
    let mut chars = identity.chars();
    let Some(first_char) = chars.next() else {
        return "<empty>".to_string();
    };
    format!("{first_char}***")
}

pub(super) fn shorten_fingerprint(fingerprint: &str) -> String {
    const MAX_VISIBLE_LEN: usize = 12;
    let mut chars = fingerprint.chars();
    let shortened: String = chars.by_ref().take(MAX_VISIBLE_LEN).collect();
    if chars.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}
