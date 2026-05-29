use super::git_config::GitConfigCredentialProvider;
use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthError, AuthErrorCode, AuthMaterial, AuthResult,
    AuthTransport, CredentialLookupKey, GitAuthProvider, HttpsAuthMaterial,
};
use dirs::home_dir;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::PathBuf;

const GIT_CREDENTIALS_PROVIDER_ID: &str = "git-credentials";

/// HTTPS credential provider backed by plaintext `git-credentials` files.
#[derive(Debug, Clone)]
pub struct GitCredentialsFileProvider {
    git_config_provider: GitConfigCredentialProvider,
    credential_paths: Vec<PathBuf>,
}

impl GitCredentialsFileProvider {
    /// Creates a provider with an explicit list of credential files.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        git_config_provider: GitConfigCredentialProvider,
        credential_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            git_config_provider,
            credential_paths,
        }
    }

    /// Creates a provider with the default set of paths (`XDG_CONFIG_HOME`, `~/.git-credentials`).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_default_paths(git_config_provider: GitConfigCredentialProvider) -> Self {
        Self::new(git_config_provider, default_credential_paths())
    }

    /// Returns the list of paths used by the provider.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn credential_paths(&self) -> &[PathBuf] {
        &self.credential_paths
    }
}

impl Default for GitCredentialsFileProvider {
    fn default() -> Self {
        Self::with_default_paths(GitConfigCredentialProvider::default())
    }
}

impl GitAuthProvider for GitCredentialsFileProvider {
    fn id(&self) -> &'static str {
        GIT_CREDENTIALS_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Https && caps.allow_https()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let lookup_key = self.git_config_provider.resolve_lookup_key(ctx)?;
        let entries = load_credential_entries(self.credential_paths())?;
        let Some(entry) = select_entry(&entries, &lookup_key) else {
            tracing::trace!(
                provider = self.id(),
                lookup = %lookup_key.redacted_target(),
                "git-credentials entry not found for lookup key"
            );
            return Ok(None);
        };

        let https_material = HttpsAuthMaterial::new(
            entry.username.clone(),
            entry.password.clone(),
            format!("git-credentials:{}:{}", entry.host, entry.username),
        );
        tracing::trace!(
            provider = self.id(),
            lookup = %lookup_key.redacted_target(),
            selected_entry = ?entry,
            material = %https_material.redacted_label(),
            "git-credentials entry selected for HTTPS auth"
        );

        Ok(Some(https_material.into_auth_material(self.id())))
    }
}

#[derive(Clone, PartialEq, Eq)]
struct GitCredentialEntry {
    host: String,
    port: Option<u16>,
    path: Option<String>,
    username: String,
    password: String,
}

impl GitCredentialEntry {
    fn match_score(&self, lookup_key: &CredentialLookupKey) -> Option<usize> {
        if !self.host.eq_ignore_ascii_case(lookup_key.host()) {
            return None;
        }

        let mut score = 1_000_usize;
        if let Some(port) = self.port {
            if lookup_key.port() != Some(port) {
                return None;
            }
            score += 100;
        }

        if let Some(username_hint) = lookup_key.username_hint() {
            if username_hint != self.username {
                return None;
            }
            score += 10;
        }

        if lookup_key.use_http_path() {
            match (self.path.as_deref(), lookup_key.path()) {
                (Some(entry_path), Some(lookup_path)) if lookup_path.starts_with(entry_path) => {
                    score += entry_path.len();
                }
                (Some(_), _) => return None,
                (None, _) => {}
            }
        }

        Some(score)
    }
}

impl fmt::Debug for GitCredentialEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitCredentialEntry")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("path", &self.path)
            .field("username", &redact_identity(&self.username))
            .field("password", &"<redacted-secret>")
            .finish()
    }
}

fn load_credential_entries(paths: &[PathBuf]) -> AuthResult<Vec<GitCredentialEntry>> {
    let mut entries = Vec::new();

    for path in paths {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(AuthError::new(
                    AuthErrorCode::NoCredentials,
                    format!(
                        "failed to read git credentials file {}: {error}",
                        path.display()
                    ),
                ))
            }
        };

        for line_result in BufReader::new(file).lines() {
            let line = line_result.map_err(|error| {
                AuthError::new(
                    AuthErrorCode::NoCredentials,
                    format!(
                        "failed to read git credentials file {}: {error}",
                        path.display()
                    ),
                )
            })?;
            let Some(entry) = parse_credential_entry(line.as_str()) else {
                continue;
            };
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn parse_credential_entry(line: &str) -> Option<GitCredentialEntry> {
    let normalized = line.trim();
    if normalized.is_empty() || normalized.starts_with('#') {
        return None;
    }

    let (_, remainder) = normalized.split_once("://")?;
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    let path = normalize_path(&remainder[authority_end..]);

    let (userinfo, host_port) = authority.rsplit_once('@')?;
    let (username, password) = userinfo.split_once(':')?;
    let (host, port) = parse_host_port(host_port)?;

    let username = username.trim();
    let password = password.trim();
    if username.is_empty() || password.is_empty() {
        return None;
    }

    Some(GitCredentialEntry {
        host,
        port,
        path,
        username: username.to_string(),
        password: password.to_string(),
    })
}

fn parse_host_port(host_port: &str) -> Option<(String, Option<u16>)> {
    let host_port = host_port.trim();
    if host_port.is_empty() {
        return None;
    }

    if let Some(stripped) = host_port.strip_prefix('[') {
        let (host, suffix) = stripped.split_once(']')?;
        let port = suffix
            .strip_prefix(':')
            .and_then(|raw_port| raw_port.parse::<u16>().ok());
        return Some((host.to_string(), port));
    }

    if let Some((host, raw_port)) = host_port.rsplit_once(':') {
        if !host.contains(':') {
            let port = raw_port.parse::<u16>().ok()?;
            return Some((host.to_string(), Some(port)));
        }
    }

    Some((host_port.to_string(), None))
}

fn normalize_path(path: &str) -> Option<String> {
    let normalized = path.trim().trim_start_matches('/').to_string();
    (!normalized.is_empty()).then_some(normalized)
}

fn select_entry<'a>(
    entries: &'a [GitCredentialEntry],
    lookup_key: &CredentialLookupKey,
) -> Option<&'a GitCredentialEntry> {
    let mut selected: Option<(&GitCredentialEntry, usize)> = None;

    for entry in entries {
        let Some(score) = entry.match_score(lookup_key) else {
            continue;
        };

        if selected
            .as_ref()
            .is_none_or(|(_, best_score)| score > *best_score)
        {
            selected = Some((entry, score));
        }
    }

    selected.map(|(entry, _)| entry)
}

fn default_credential_paths() -> Vec<PathBuf> {
    if let Some(path_override) = std::env::var_os("GIT_CREDENTIALS_PATH") {
        return vec![PathBuf::from(path_override)];
    }

    let mut paths = Vec::new();

    if let Some(xdg_config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        push_unique_path(
            &mut paths,
            PathBuf::from(xdg_config_home)
                .join("git")
                .join("credentials"),
        );
    } else if let Some(home) = home_dir() {
        push_unique_path(
            &mut paths,
            home.join(".config").join("git").join("credentials"),
        );
    }

    if let Some(home) = home_dir() {
        push_unique_path(&mut paths, home.join(".git-credentials"));
    }

    paths
}

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if paths.iter().any(|existing| existing == &candidate) {
        return;
    }
    paths.push(candidate);
}

fn redact_identity(identity: &str) -> String {
    let mut chars = identity.chars();
    let Some(first_char) = chars.next() else {
        return "<empty>".to_string();
    };
    format!("{first_char}***")
}
