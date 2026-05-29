use super::paths::{expand_config_path, expand_include_paths, normalize_path};
use super::SshConfigResolved;
use crate::git_manager::auth::{AuthContext, AuthError, AuthErrorCode, AuthResult};
use glob::Pattern;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(super) struct ResolveState {
    resolved: SshConfigResolved,
    user_locked: bool,
    port_locked: bool,
    apply_flags: ResolveApplyFlags,
}

#[derive(Debug, Clone, Default)]
struct ResolveApplyFlags {
    hostname_set_from_config: bool,
    identities_only_set: bool,
}

impl ResolveState {
    pub(super) fn new(ctx: &AuthContext) -> Self {
        let mut resolved = SshConfigResolved::new(ctx.remote_host());
        let user_locked = ctx.username_hint().is_some();
        if let Some(username_hint) = ctx.username_hint() {
            resolved.user = Some(username_hint.to_string());
        }
        let port_locked = ctx.remote_port().is_some();
        if let Some(port) = ctx.remote_port() {
            resolved.port = Some(port);
        }

        Self {
            resolved,
            user_locked,
            port_locked,
            apply_flags: ResolveApplyFlags::default(),
        }
    }

    fn set_hostname(&mut self, hostname: String) {
        if self.apply_flags.hostname_set_from_config {
            return;
        }
        self.resolved.hostname = hostname;
        self.apply_flags.hostname_set_from_config = true;
    }

    fn set_user(&mut self, user: String) {
        if self.user_locked || self.resolved.user.is_some() {
            return;
        }
        self.resolved.user = Some(user);
    }

    const fn set_port(&mut self, port: u16) {
        if self.port_locked || self.resolved.port.is_some() {
            return;
        }
        self.resolved.port = Some(port);
    }

    fn push_identity_file(&mut self, identity_file: PathBuf) {
        if self.resolved.identity_files.contains(&identity_file) {
            return;
        }
        self.resolved.identity_files.push(identity_file);
    }

    const fn set_identities_only(&mut self, identities_only: bool) {
        if self.apply_flags.identities_only_set {
            return;
        }
        self.resolved.identities_only = identities_only;
        self.apply_flags.identities_only_set = true;
    }

    pub(super) fn into_resolved(self) -> SshConfigResolved {
        self.resolved
    }
}

#[derive(Debug, Default)]
pub(super) struct ParseVisitState {
    visited_files: HashSet<PathBuf>,
}

impl ParseVisitState {
    fn mark_visited(&mut self, file_path: &Path) -> bool {
        self.visited_files.insert(normalize_path(file_path))
    }
}

#[derive(Debug, Clone)]
pub(super) struct DirectiveSource {
    file_path: PathBuf,
    line: usize,
}

impl DirectiveSource {
    fn new(file_path: &Path, line: usize) -> Self {
        Self {
            file_path: file_path.to_path_buf(),
            line,
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedDirective {
    keyword: String,
    values: Vec<String>,
}

pub(super) fn parse_config_file(
    config_path: &Path,
    target_host: &str,
    state: &mut ResolveState,
    visit_state: &mut ParseVisitState,
) -> AuthResult<()> {
    if !visit_state.mark_visited(config_path) {
        return Ok(());
    }

    let file = match File::open(config_path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AuthError::new(
                AuthErrorCode::NoCredentials,
                format!(
                    "failed to read ssh config `{}`: {error}",
                    config_path.display()
                ),
            ))
        }
    };

    let mut host_patterns: Option<Vec<String>> = None;
    for (index, line_result) in BufReader::new(file).lines().enumerate() {
        let line_number = index + 1;
        let line = line_result.map_err(|error| {
            AuthError::new(
                AuthErrorCode::NoCredentials,
                format!(
                    "failed to read ssh config `{}` line {line_number}: {error}",
                    config_path.display()
                ),
            )
        })?;

        let Some(parsed) = parse_directive_line(&line) else {
            continue;
        };
        let source = DirectiveSource::new(config_path, line_number);

        if parsed.keyword.eq_ignore_ascii_case("host") {
            validate_host_patterns(&parsed.values, &source)?;
            host_patterns = Some(parsed.values);
            continue;
        }

        let block_matches = host_block_matches(target_host, host_patterns.as_deref());
        if !block_matches {
            continue;
        }

        apply_directive(
            &parsed,
            &source,
            target_host,
            state,
            visit_state,
            config_path,
        )?;
    }

    Ok(())
}

fn apply_directive(
    directive: &ParsedDirective,
    source: &DirectiveSource,
    target_host: &str,
    state: &mut ResolveState,
    visit_state: &mut ParseVisitState,
    config_path: &Path,
) -> AuthResult<()> {
    let keyword = directive.keyword.to_ascii_lowercase();
    match keyword.as_str() {
        "hostname" => {
            let hostname = require_single_value(directive, source)?;
            state.set_hostname(hostname.to_string());
            Ok(())
        }
        "user" => {
            let user = require_single_value(directive, source)?;
            state.set_user(user.to_string());
            Ok(())
        }
        "port" => {
            let raw_port = require_single_value(directive, source)?;
            let port = raw_port.parse::<u16>().map_err(|_| {
                unsupported_directive_error(
                    source,
                    format!("Port expects u16 value, got `{raw_port}`"),
                )
            })?;
            if port == 0 {
                return Err(unsupported_directive_error(
                    source,
                    "Port value must be greater than zero".to_string(),
                ));
            }
            state.set_port(port);
            Ok(())
        }
        "identityfile" => {
            let raw_identity = require_single_value(directive, source)?;
            let parent_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
            let identity_path = expand_config_path(raw_identity, parent_dir);
            state.push_identity_file(identity_path);
            Ok(())
        }
        "identitiesonly" => {
            let raw_value = require_single_value(directive, source)?;
            let identities_only = parse_yes_no(raw_value).ok_or_else(|| {
                unsupported_directive_error(
                    source,
                    format!("IdentitiesOnly expects yes/no, got `{raw_value}`"),
                )
            })?;
            state.set_identities_only(identities_only);
            Ok(())
        }
        "include" => {
            let include_values = require_values(directive, source)?;
            let parent_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
            for raw_include in include_values {
                for include_path in expand_include_paths(raw_include, parent_dir, source)? {
                    parse_config_file(&include_path, target_host, state, visit_state)?;
                }
            }
            Ok(())
        }
        unsupported => Err(unsupported_directive_error(
            source,
            format!("directive `{unsupported}` is not supported by SSH MVP subset"),
        )),
    }
}

fn parse_directive_line(line: &str) -> Option<ParsedDirective> {
    let line_without_comment = line.split('#').next().unwrap_or("").trim();
    if line_without_comment.is_empty() {
        return None;
    }

    let mut tokens = line_without_comment.split_whitespace();
    let first = tokens.next()?;
    let mut values: Vec<String> = tokens.map(str::to_owned).collect();
    let keyword = if let Some((key, inline_value)) = first.split_once('=') {
        if !inline_value.is_empty() {
            values.insert(0, inline_value.to_string());
        }
        key.to_string()
    } else {
        first.to_string()
    };

    Some(ParsedDirective { keyword, values })
}

fn validate_host_patterns(patterns: &[String], source: &DirectiveSource) -> AuthResult<()> {
    if patterns.is_empty() {
        return Err(unsupported_directive_error(
            source,
            "Host directive requires at least one pattern".to_string(),
        ));
    }

    for pattern in patterns {
        let pattern = pattern.strip_prefix('!').unwrap_or(pattern);
        if pattern.is_empty() {
            return Err(unsupported_directive_error(
                source,
                "Host pattern must not be empty".to_string(),
            ));
        }

        Pattern::new(pattern).map_err(|error| {
            unsupported_directive_error(
                source,
                format!("invalid Host pattern `{pattern}`: {error}"),
            )
        })?;
    }

    Ok(())
}

fn host_block_matches(target_host: &str, host_patterns: Option<&[String]>) -> bool {
    let Some(host_patterns) = host_patterns else {
        return true;
    };
    let mut positive_match = false;
    for raw_pattern in host_patterns {
        if let Some(negated_pattern) = raw_pattern.strip_prefix('!') {
            if Pattern::new(negated_pattern).is_ok_and(|pattern| pattern.matches(target_host)) {
                return false;
            }
            continue;
        }

        if Pattern::new(raw_pattern).is_ok_and(|pattern| pattern.matches(target_host)) {
            positive_match = true;
        }
    }

    positive_match
}

fn require_single_value<'a>(
    directive: &'a ParsedDirective,
    source: &DirectiveSource,
) -> AuthResult<&'a str> {
    let values = require_values(directive, source)?;
    if values.len() != 1 {
        return Err(unsupported_directive_error(
            source,
            format!(
                "directive `{}` expects exactly one value",
                directive.keyword
            ),
        ));
    }
    Ok(values[0].as_str())
}

fn require_values<'a>(
    directive: &'a ParsedDirective,
    source: &DirectiveSource,
) -> AuthResult<&'a [String]> {
    if directive.values.is_empty() {
        return Err(unsupported_directive_error(
            source,
            format!(
                "directive `{}` expects at least one value",
                directive.keyword
            ),
        ));
    }
    Ok(&directive.values)
}

fn parse_yes_no(raw_value: &str) -> Option<bool> {
    match raw_value.to_ascii_lowercase().as_str() {
        "yes" | "true" | "on" | "1" => Some(true),
        "no" | "false" | "off" | "0" => Some(false),
        _ => None,
    }
}

pub(super) fn unsupported_directive_error(
    source: &DirectiveSource,
    details: impl Into<String>,
) -> AuthError {
    let details = details.into();
    AuthError::new(
        AuthErrorCode::UnsupportedSshDirective,
        format!("{details} ({}:{})", source.file_path.display(), source.line),
    )
}
