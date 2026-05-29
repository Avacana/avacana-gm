use super::parser::netrc_parse_error;
use crate::git_manager::auth::{AuthResult, CredentialLookupKey, HttpsAuthMaterial, NetrcEntry};
#[cfg(not(target_os = "windows"))]
use dirs::home_dir;
use std::path::{Path, PathBuf};

pub(super) fn select_entry<'a>(
    entries: &'a [NetrcEntry],
    lookup_key: &CredentialLookupKey,
) -> Option<&'a NetrcEntry> {
    let username_hint = lookup_key.username_hint();

    for machine_candidate in lookup_key.machine_candidates() {
        if let Some(entry) = select_entry_for_machine(entries, &machine_candidate, username_hint) {
            return Some(entry);
        }
    }

    select_default_entry(entries, username_hint)
}

fn select_entry_for_machine<'a>(
    entries: &'a [NetrcEntry],
    machine_candidate: &str,
    username_hint: Option<&str>,
) -> Option<&'a NetrcEntry> {
    let mut fallback_without_login: Option<&NetrcEntry> = None;
    let mut fallback_any: Option<&NetrcEntry> = None;

    for entry in entries {
        let Some(machine) = entry.machine() else {
            continue;
        };
        if !machine.eq_ignore_ascii_case(machine_candidate) {
            continue;
        }

        match (entry.login(), username_hint) {
            (Some(login), Some(hint)) if login == hint => {
                return Some(entry);
            }
            (None, Some(_)) => {
                if fallback_without_login.is_none() {
                    fallback_without_login = Some(entry);
                }
            }
            (_, Some(_)) => {
                if fallback_any.is_none() {
                    fallback_any = Some(entry);
                }
            }
            (_, None) => {
                return Some(entry);
            }
        }
    }

    fallback_without_login.or(fallback_any)
}

fn select_default_entry<'a>(
    entries: &'a [NetrcEntry],
    username_hint: Option<&str>,
) -> Option<&'a NetrcEntry> {
    let mut fallback_without_login: Option<&NetrcEntry> = None;
    let mut fallback_any: Option<&NetrcEntry> = None;

    for entry in entries {
        if !entry.is_default() {
            continue;
        }

        match (entry.login(), username_hint) {
            (Some(login), Some(hint)) if login == hint => {
                return Some(entry);
            }
            (None, Some(_)) => {
                if fallback_without_login.is_none() {
                    fallback_without_login = Some(entry);
                }
            }
            (_, Some(_)) => {
                if fallback_any.is_none() {
                    fallback_any = Some(entry);
                }
            }
            (_, None) => {
                return Some(entry);
            }
        }
    }

    fallback_without_login.or(fallback_any)
}

pub(super) fn build_https_material(
    entry: &NetrcEntry,
    lookup_key: &CredentialLookupKey,
    netrc_path: &Path,
) -> AuthResult<HttpsAuthMaterial> {
    let username = match (entry.login(), lookup_key.username_hint()) {
        (Some(login), _) => login.to_string(),
        (None, Some(username_hint)) => username_hint.to_string(),
        (None, None) => {
            return Err(netrc_parse_error(
                netrc_path,
                0,
                "netrc entry is missing 'login' and no username hint was provided",
            ));
        }
    };

    let Some(password) = entry.password() else {
        return Err(netrc_parse_error(
            netrc_path,
            0,
            "netrc entry is missing 'password'",
        ));
    };

    let machine = entry.machine().unwrap_or("default");
    Ok(HttpsAuthMaterial::new(
        username.clone(),
        password.to_string(),
        format!("netrc:{machine}:{username}"),
    ))
}

pub(super) fn default_netrc_path() -> Option<PathBuf> {
    if let Some(netrc_from_env) = std::env::var_os("NETRC") {
        return Some(PathBuf::from(netrc_from_env));
    }

    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|home| home.join("_netrc"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        home_dir().map(|home| home.join(".netrc"))
    }
}
