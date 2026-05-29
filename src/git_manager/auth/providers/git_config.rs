use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthError, AuthErrorCode, AuthMaterial, AuthMaterialKind,
    AuthResult, AuthTransport, CredentialLookupKey, GitAuthProvider,
};
use dirs::home_dir;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

const GIT_CONFIG_PROVIDER_ID: &str = "git-config";

/// Provider of username/useHttpPath hints from git-config for HTTPS lookup.
#[derive(Debug, Clone)]
pub struct GitConfigCredentialProvider {
    config_path: Option<PathBuf>,
}

impl GitConfigCredentialProvider {
    /// Creates a provider with an explicit path to the git-config file.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(config_path: Option<PathBuf>) -> Self {
        Self { config_path }
    }

    /// Creates a provider using the default path to the global git-config.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_default_path() -> Self {
        Self::new(default_git_config_path())
    }

    /// Returns the path to the git-config file, if one is set.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Resolves the lookup key from the URL and git-config hints.
    ///
    /// Username hint priority: URL/context (`AuthContext::username_hint`) > git-config.
    ///
    /// # Errors
    /// Returns a typed error on I/O failures while reading git-config.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(target = %ctx.redacted_target()))
    )]
    pub fn resolve_lookup_key(&self, ctx: &AuthContext) -> AuthResult<CredentialLookupKey> {
        let mut lookup_key = CredentialLookupKey::from_context(ctx);

        let Some(config_path) = self.config_path.as_deref() else {
            return Ok(lookup_key);
        };

        let hints = parse_config_hints(config_path, &lookup_key)?;
        if let Some(use_http_path) = hints.use_http_path {
            lookup_key = lookup_key.with_use_http_path(use_http_path);
        }

        if lookup_key.username_hint().is_none() {
            if let Some(username) = hints.username {
                lookup_key = lookup_key.with_username_hint(username);
            }
        }

        tracing::trace!(
            lookup = %lookup_key.redacted_target(),
            use_http_path = lookup_key.use_http_path(),
            "resolved credential lookup key from URL/git-config hints"
        );

        Ok(lookup_key)
    }

    /// Resolves the username hint from the URL/git-config context.
    ///
    /// # Errors
    /// Returns a typed error on I/O failures while reading git-config.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(target = %ctx.redacted_target()))
    )]
    pub fn resolve_username_hint(&self, ctx: &AuthContext) -> AuthResult<Option<String>> {
        let lookup_key = self.resolve_lookup_key(ctx)?;
        Ok(lookup_key.username_hint().map(str::to_string))
    }
}

impl Default for GitConfigCredentialProvider {
    fn default() -> Self {
        Self::with_default_path()
    }
}

impl GitAuthProvider for GitConfigCredentialProvider {
    fn id(&self) -> &'static str {
        GIT_CONFIG_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Https && caps.allow_https()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let Some(username_hint) = self.resolve_username_hint(ctx)? else {
            return Ok(None);
        };

        Ok(Some(AuthMaterial::without_secret(
            self.id(),
            AuthMaterialKind::UsernameOnly,
            Some(username_hint.clone()),
            format!("git-config-username:{username_hint}"),
        )))
    }
}

#[derive(Debug, Clone, Default)]
struct CredentialHints {
    username: Option<String>,
    use_http_path: Option<bool>,
}

impl CredentialHints {
    fn apply_assignment(&mut self, key: &str, value: &str) {
        if key.eq_ignore_ascii_case("username") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                self.username = Some(trimmed.to_string());
            }
            return;
        }

        if key.eq_ignore_ascii_case("useHttpPath") {
            self.use_http_path = parse_git_bool(value);
        }
    }

    fn merge_from(&mut self, other: Self) {
        if let Some(username) = other.username {
            self.username = Some(username);
        }
        if let Some(use_http_path) = other.use_http_path {
            self.use_http_path = Some(use_http_path);
        }
    }
}

#[derive(Debug, Clone)]
enum SectionState {
    GlobalCredential,
    UrlScoped(CredentialUrlScope),
    Other,
}

#[derive(Debug, Clone)]
struct CredentialUrlScope {
    host: String,
    port: Option<u16>,
    path_prefix: Option<String>,
}

impl CredentialUrlScope {
    fn parse(raw_subsection: &str) -> Option<Self> {
        let trimmed = trim_wrapping_quotes(raw_subsection.trim());
        let (_, target) = trimmed.split_once("://")?;
        let (authority, path) = target
            .split_once('/')
            .map_or((target, None), |(authority, path)| (authority, Some(path)));
        let authority = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let (host, port) = parse_host_port(authority)?;

        Some(Self {
            host,
            port,
            path_prefix: normalize_lookup_path(path),
        })
    }

    fn match_score(&self, lookup_key: &CredentialLookupKey) -> Option<usize> {
        if !self.host.eq_ignore_ascii_case(lookup_key.host()) {
            return None;
        }

        if let Some(scope_port) = self.port {
            if lookup_key.port() != Some(scope_port) {
                return None;
            }
        }

        let mut score = 1000;
        if self.port.is_some() {
            score += 100;
        }

        if let Some(scope_path) = self.path_prefix.as_deref() {
            let lookup_path = lookup_key.path()?;
            if !lookup_path.starts_with(scope_path) {
                return None;
            }
            score += scope_path.len();
        }

        Some(score)
    }
}

fn parse_config_hints(
    config_path: &Path,
    lookup_key: &CredentialLookupKey,
) -> AuthResult<CredentialHints> {
    let file = match File::open(config_path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(CredentialHints::default());
        }
        Err(error) => {
            return Err(config_io_error(config_path, &error));
        }
    };

    let reader = BufReader::new(file);
    let mut active_section = SectionState::Other;
    let mut global_hints = CredentialHints::default();
    let mut scoped_hints = CredentialHints::default();
    let mut best_scope_score: Option<usize> = None;

    for line_result in reader.lines() {
        let line = line_result.map_err(|error| config_io_error(config_path, &error))?;
        let normalized = strip_inline_comment(line.trim());
        if normalized.is_empty() {
            continue;
        }

        if normalized.starts_with('[') && normalized.ends_with(']') {
            active_section = parse_section_header(normalized);
            continue;
        }

        let Some((key, value)) = parse_assignment(normalized) else {
            continue;
        };

        match &active_section {
            SectionState::GlobalCredential => {
                global_hints.apply_assignment(key, value);
            }
            SectionState::UrlScoped(url_scope) => {
                let Some(score) = url_scope.match_score(lookup_key) else {
                    continue;
                };

                match best_scope_score {
                    None => {
                        best_scope_score = Some(score);
                        scoped_hints.apply_assignment(key, value);
                    }
                    Some(best_score) if score > best_score => {
                        best_scope_score = Some(score);
                        scoped_hints = CredentialHints::default();
                        scoped_hints.apply_assignment(key, value);
                    }
                    Some(best_score) if score == best_score => {
                        scoped_hints.apply_assignment(key, value);
                    }
                    Some(_) => {}
                }
            }
            SectionState::Other => {}
        }
    }

    global_hints.merge_from(scoped_hints);
    Ok(global_hints)
}

fn parse_section_header(header: &str) -> SectionState {
    let inner = header
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(str::trim)
        .unwrap_or_default();
    let mut parts = inner.splitn(2, char::is_whitespace);
    let Some(section_name) = parts.next() else {
        return SectionState::Other;
    };
    if !section_name.eq_ignore_ascii_case("credential") {
        return SectionState::Other;
    }

    let Some(raw_subsection) = parts.next() else {
        return SectionState::GlobalCredential;
    };

    CredentialUrlScope::parse(raw_subsection).map_or(SectionState::Other, SectionState::UrlScoped)
}

fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    if let Some((key, value)) = line.split_once('=') {
        let key = key.trim();
        let value = value.trim();
        return (!key.is_empty()).then_some((key, value));
    }

    let mut parts = line.split_whitespace();
    let key = parts.next()?;
    let value = parts.next().unwrap_or_default();
    Some((key.trim(), value.trim()))
}

fn parse_git_bool(raw_value: &str) -> Option<bool> {
    let normalized = raw_value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn parse_host_port(authority: &str) -> Option<(String, Option<u16>)> {
    let normalized_authority = authority.trim();
    if normalized_authority.is_empty() {
        return None;
    }

    if let Some((host, port_raw)) = normalized_authority.rsplit_once(':') {
        if host.contains(':') {
            return Some((normalized_authority.to_string(), None));
        }
        if let Ok(port) = port_raw.parse::<u16>() {
            return Some((host.to_string(), Some(port)));
        }
    }

    Some((normalized_authority.to_string(), None))
}

fn normalize_lookup_path(path: Option<&str>) -> Option<String> {
    let path = path?.trim().trim_start_matches('/');
    (!path.is_empty()).then_some(path.to_string())
}

fn strip_inline_comment(line: &str) -> &str {
    let mut end = line.len();
    if let Some(index) = line.find('#') {
        end = end.min(index);
    }
    if let Some(index) = line.find(';') {
        end = end.min(index);
    }
    line[..end].trim()
}

fn trim_wrapping_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

fn config_io_error(config_path: &Path, error: &std::io::Error) -> AuthError {
    AuthError::new(
        AuthErrorCode::NoCredentials,
        format!(
            "failed to read git-config credential hints from {}: {error}",
            config_path.display()
        ),
    )
}

fn default_git_config_path() -> Option<PathBuf> {
    if let Some(config_override) = std::env::var_os("GIT_CONFIG_GLOBAL") {
        return Some(PathBuf::from(config_override));
    }

    home_dir().map(|home| home.join(".gitconfig"))
}
