use crate::git_manager::auth::{AuthError, AuthErrorCode, AuthResult};
#[cfg(not(target_os = "windows"))]
use dirs::home_dir;
use glob::Pattern;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

const DEFAULT_SSH_PORT: u16 = 22;

/// Policy for verifying the SSH host key inside the transport callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostKeyPolicy {
    accept_new_host: bool,
}

impl HostKeyPolicy {
    /// Returns a strict policy (`accept_new_host=false`).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn strict() -> Self {
        Self {
            accept_new_host: false,
        }
    }

    /// Returns a policy with `accept_new_host` enabled.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn accept_new_host() -> Self {
        Self {
            accept_new_host: true,
        }
    }

    /// Returns a copy of the policy with the given `accept_new_host` state.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_accept_new_host(mut self, accept_new_host: bool) -> Self {
        self.accept_new_host = accept_new_host;
        self
    }

    /// Returns `true` if explicit TOFU is allowed (`accept_new_host`).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn accept_new_host_enabled(self) -> bool {
        self.accept_new_host
    }
}

impl Default for HostKeyPolicy {
    fn default() -> Self {
        Self::strict()
    }
}

/// Outcome of applying the host key policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyPolicyResult {
    /// The presented host key matched an entry in `known_hosts`.
    TrustedMatch,
    /// The host key was accepted via the explicit `accept_new_host=true` opt-in.
    AcceptedByPolicy,
}

/// `known_hosts` verifier for the SSH transport.
#[derive(Debug, Clone)]
pub struct KnownHostsVerifier {
    known_hosts_path: Option<PathBuf>,
    policy: HostKeyPolicy,
}

impl KnownHostsVerifier {
    /// Creates a verifier with an explicit path and policy.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(known_hosts_path: Option<PathBuf>, policy: HostKeyPolicy) -> Self {
        Self {
            known_hosts_path,
            policy,
        }
    }

    /// Creates a verifier using the default user `known_hosts` file.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_default_path(policy: HostKeyPolicy) -> Self {
        Self::new(default_known_hosts_path(), policy)
    }

    /// Returns the path to `known_hosts`, if one is set.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn known_hosts_path(&self) -> Option<&Path> {
        self.known_hosts_path.as_deref()
    }

    /// Returns the current host key verification policy.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn policy(&self) -> HostKeyPolicy {
        self.policy
    }

    /// Verifies the presented host key against `known_hosts`.
    ///
    /// # Errors
    /// Returns `AUTH_HOSTKEY_UNKNOWN` when no entry is found (strict policy)
    /// and `AUTH_HOSTKEY_MISMATCH` when the key does not match for a known host.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(host = host, port = port))
    )]
    pub fn verify_host_key(
        &self,
        host: &str,
        port: u16,
        key_type: &str,
        key_blob_base64: &str,
    ) -> AuthResult<HostKeyPolicyResult> {
        let entries = read_known_hosts_entries(self.known_hosts_path.as_deref())?;
        let host_entries: Vec<&KnownHostsEntry> = entries
            .iter()
            .filter(|entry| entry.matches_host(host, port))
            .collect();

        if host_entries
            .iter()
            .any(|entry| entry.matches_key(key_type, key_blob_base64))
        {
            return Ok(HostKeyPolicyResult::TrustedMatch);
        }

        if host_entries.is_empty() {
            if self.policy.accept_new_host_enabled() {
                return Ok(HostKeyPolicyResult::AcceptedByPolicy);
            }
            return Err(AuthError::new(
                AuthErrorCode::HostKeyUnknown,
                format!(
                    "host `{host}:{port}` is missing in known_hosts `{}`",
                    known_hosts_label(self.known_hosts_path.as_deref())
                ),
            ));
        }

        Err(AuthError::new(
            AuthErrorCode::HostKeyMismatch,
            format!(
                "host key mismatch for `{host}:{port}` in known_hosts `{}`",
                known_hosts_label(self.known_hosts_path.as_deref())
            ),
        ))
    }
}

impl Default for KnownHostsVerifier {
    fn default() -> Self {
        Self::with_default_path(HostKeyPolicy::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KnownHostsEntry {
    host_patterns: Vec<String>,
    key_type: String,
    key_blob_base64: String,
}

impl KnownHostsEntry {
    fn matches_host(&self, host: &str, port: u16) -> bool {
        let mut positive_match = false;
        for raw_pattern in &self.host_patterns {
            if raw_pattern.starts_with('|') {
                continue;
            }

            if let Some(negated_pattern) = raw_pattern.strip_prefix('!') {
                if known_host_pattern_matches(host, port, negated_pattern) {
                    return false;
                }
                continue;
            }

            if known_host_pattern_matches(host, port, raw_pattern) {
                positive_match = true;
            }
        }

        positive_match
    }

    fn matches_key(&self, key_type: &str, key_blob_base64: &str) -> bool {
        self.key_type == key_type && self.key_blob_base64 == key_blob_base64
    }
}

fn read_known_hosts_entries(known_hosts_path: Option<&Path>) -> AuthResult<Vec<KnownHostsEntry>> {
    let Some(known_hosts_path) = known_hosts_path else {
        return Ok(Vec::new());
    };

    let file = match File::open(known_hosts_path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(AuthError::new(
                AuthErrorCode::NoCredentials,
                format!(
                    "failed to read known_hosts `{}`: {error}",
                    known_hosts_path.display()
                ),
            ))
        }
    };

    let mut entries = Vec::new();
    for line_result in BufReader::new(file).lines() {
        let line = line_result.map_err(|error| {
            AuthError::new(
                AuthErrorCode::NoCredentials,
                format!(
                    "failed to read known_hosts `{}`: {error}",
                    known_hosts_path.display()
                ),
            )
        })?;

        let Some(entry) = parse_known_hosts_line(&line) else {
            continue;
        };
        entries.push(entry);
    }

    Ok(entries)
}

fn parse_known_hosts_line(line: &str) -> Option<KnownHostsEntry> {
    let line_without_comment = line.split('#').next().unwrap_or("").trim();
    if line_without_comment.is_empty() {
        return None;
    }

    let mut tokens: Vec<&str> = line_without_comment.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    if tokens.first().is_some_and(|first| first.starts_with('@')) {
        let _ = tokens.remove(0);
    }
    if tokens.len() < 3 {
        return None;
    }

    let host_patterns = tokens[0]
        .split(',')
        .filter(|pattern| !pattern.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if host_patterns.is_empty() {
        return None;
    }

    Some(KnownHostsEntry {
        host_patterns,
        key_type: tokens[1].to_string(),
        key_blob_base64: tokens[2].to_string(),
    })
}

fn known_host_pattern_matches(host: &str, port: u16, pattern: &str) -> bool {
    if let Some((host_pattern, pattern_port)) = parse_bracket_host_pattern(pattern) {
        return pattern_port == port && glob_matches(host_pattern, host);
    }

    if port != DEFAULT_SSH_PORT {
        return false;
    }
    glob_matches(pattern, host)
}

fn parse_bracket_host_pattern(pattern: &str) -> Option<(&str, u16)> {
    if !pattern.starts_with('[') {
        return None;
    }

    let closing_bracket = pattern.find("]:")?;
    let host_pattern = &pattern[1..closing_bracket];
    let raw_port = &pattern[(closing_bracket + 2)..];
    let port = raw_port.parse::<u16>().ok()?;
    Some((host_pattern, port))
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    Pattern::new(pattern).is_ok_and(|glob| glob.matches(value))
}

fn known_hosts_label(known_hosts_path: Option<&Path>) -> String {
    known_hosts_path.map_or_else(
        || "<default-known-hosts>".to_string(),
        |path| path.display().to_string(),
    )
}

fn default_known_hosts_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|home| home.join(".ssh").join("known_hosts"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        home_dir().map(|home| home.join(".ssh").join("known_hosts"))
    }
}
