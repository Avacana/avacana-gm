//! Shared transport/auth/url helpers for `GitManager` remote operations.

use crate::git_manager::auth::{AuthContext, AuthTransport};
use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{FetchOptions, ProxyOptions, PushOptions, RemoteRedirect};

const ENV_TRANSPORT_PROXY_URL: &str = "AVACANA_GM_GIT_PROXY_URL";
const ENV_TRANSPORT_PROXY_MODE: &str = "AVACANA_GM_GIT_PROXY_MODE";
const ENV_TRANSPORT_REMOTE_REDIRECT: &str = "AVACANA_GM_GIT_REMOTE_REDIRECT";

pub(super) fn build_auth_context_from_remote_url(
    operation: &str,
    remote_url: &str,
) -> GitResult<AuthContext> {
    let parsed_remote = parse_remote_url(remote_url)?;
    let mut auth_context = AuthContext::new(
        operation.to_string(),
        parsed_remote.transport,
        parsed_remote.host,
    );

    if let Some(remote_port) = parsed_remote.port {
        auth_context = auth_context.with_remote_port(remote_port);
    }
    if let Some(remote_path) = parsed_remote.path {
        auth_context = auth_context.with_remote_path(remote_path);
    }
    if let Some(username_hint) = parsed_remote.username_hint {
        auth_context = auth_context.with_username_hint(username_hint);
    }

    Ok(auth_context)
}

pub(super) fn is_local_remote_url(remote_url: &str) -> bool {
    let remote_url = remote_url.trim();
    remote_url
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
        || looks_like_local_path(remote_url)
}

pub(super) fn apply_fetch_network_options(fetch_options: &mut FetchOptions<'_>) {
    fetch_options.follow_redirects(resolve_remote_redirect_policy());
    if let Some(proxy_options) = build_proxy_options() {
        fetch_options.proxy_options(proxy_options);
    }
}

pub(super) fn apply_push_network_options(push_options: &mut PushOptions<'_>) {
    push_options.follow_redirects(resolve_remote_redirect_policy());
    if let Some(proxy_options) = build_proxy_options() {
        push_options.proxy_options(proxy_options);
    }
}

pub(super) fn connect_proxy_options<'callbacks>() -> Option<ProxyOptions<'callbacks>> {
    build_proxy_options()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRemote {
    transport: AuthTransport,
    host: String,
    port: Option<u16>,
    path: Option<String>,
    username_hint: Option<String>,
}

fn parse_remote_url(remote_url: &str) -> GitResult<ParsedRemote> {
    let remote_url = remote_url.trim();
    if remote_url.is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "remote URL is empty",
        ));
    }

    if let Some((scheme, remainder)) = remote_url.split_once("://") {
        return parse_remote_with_scheme(scheme, remainder);
    }

    if looks_like_local_path(remote_url) {
        return Ok(ParsedRemote {
            transport: AuthTransport::Https,
            host: "localhost".to_string(),
            port: None,
            path: normalize_remote_path(remote_url),
            username_hint: None,
        });
    }

    parse_scp_like_remote(remote_url)
}

fn parse_remote_with_scheme(scheme: &str, remainder: &str) -> GitResult<ParsedRemote> {
    match scheme.to_ascii_lowercase().as_str() {
        "https" => parse_standard_remote(AuthTransport::Https, remainder),
        "ssh" => parse_standard_remote(AuthTransport::Ssh, remainder),
        "file" => Ok(parse_file_remote(remainder)),
        unsupported_scheme => Err(GitError::new(
            GitErrorCode::TransportFailure,
            format!("unsupported remote URL scheme `{unsupported_scheme}`"),
        )),
    }
}

fn parse_standard_remote(transport: AuthTransport, remainder: &str) -> GitResult<ParsedRemote> {
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    if authority.is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "remote URL authority is empty",
        ));
    }

    let path = normalize_remote_path(&remainder[authority_end..]);
    let (username_hint, host_port) = split_userinfo(authority);
    let (host, port) = parse_host_port(host_port)?;

    Ok(ParsedRemote {
        transport,
        host,
        port,
        path,
        username_hint,
    })
}

fn parse_file_remote(remainder: &str) -> ParsedRemote {
    let (host, path) = if remainder.starts_with('/') {
        ("localhost".to_string(), normalize_remote_path(remainder))
    } else {
        let authority_end = remainder.find('/').unwrap_or(remainder.len());
        let host = remainder[..authority_end].to_string();
        let path = normalize_remote_path(&remainder[authority_end..]);
        (
            if host.is_empty() {
                "localhost".to_string()
            } else {
                host
            },
            path,
        )
    };

    ParsedRemote {
        transport: AuthTransport::Https,
        host,
        port: None,
        path,
        username_hint: None,
    }
}

fn parse_scp_like_remote(remote_url: &str) -> GitResult<ParsedRemote> {
    let Some((host_part, path_part)) = remote_url.split_once(':') else {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            format!("unsupported remote URL format `{remote_url}`"),
        ));
    };

    if host_part.contains('/') || path_part.is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            format!("unsupported remote URL format `{remote_url}`"),
        ));
    }

    let (username_hint, host_part) = split_userinfo(host_part);
    let (host, port) = parse_host_port(host_part)?;
    Ok(ParsedRemote {
        transport: AuthTransport::Ssh,
        host,
        port,
        path: normalize_remote_path(path_part),
        username_hint,
    })
}

fn split_userinfo(authority: &str) -> (Option<String>, &str) {
    let Some((userinfo, host_port)) = authority.rsplit_once('@') else {
        return (None, authority);
    };

    let username_hint = userinfo
        .split(':')
        .next()
        .and_then(non_empty)
        .map(str::to_owned);
    (username_hint, host_port)
}

fn parse_host_port(host_port: &str) -> GitResult<(String, Option<u16>)> {
    let host_port = host_port.trim();
    if host_port.is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "remote host is empty",
        ));
    }

    if let Some(stripped) = host_port.strip_prefix('[') {
        let Some((host, suffix)) = stripped.split_once(']') else {
            return Err(GitError::new(
                GitErrorCode::TransportFailure,
                format!("invalid bracketed host `{host_port}`"),
            ));
        };
        let port = suffix.strip_prefix(':').map(parse_port).transpose()?;
        return Ok((host.to_string(), port));
    }

    if let Some((host, raw_port)) = host_port.rsplit_once(':') {
        if !host.contains(':') {
            let port = parse_port(raw_port)?;
            return Ok((host.to_string(), Some(port)));
        }
    }

    Ok((host_port.to_string(), None))
}

fn parse_port(raw_port: &str) -> GitResult<u16> {
    raw_port.parse::<u16>().map_err(|_| {
        GitError::new(
            GitErrorCode::TransportFailure,
            format!("invalid remote port `{raw_port}`"),
        )
    })
}

fn looks_like_local_path(value: &str) -> bool {
    value.starts_with('/') || value.starts_with("./") || value.starts_with("../")
}

fn normalize_remote_path(path: &str) -> Option<String> {
    let normalized = path.trim().trim_start_matches('/');
    (!normalized.is_empty()).then(|| normalized.to_string())
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn build_proxy_options<'callbacks>() -> Option<ProxyOptions<'callbacks>> {
    if let Some(proxy_url) = read_non_empty_env(ENV_TRANSPORT_PROXY_URL) {
        let mut proxy_options = ProxyOptions::new();
        proxy_options.url(&proxy_url);
        return Some(proxy_options);
    }

    if read_non_empty_env(ENV_TRANSPORT_PROXY_MODE)
        .is_some_and(|mode| mode.eq_ignore_ascii_case("auto"))
    {
        let mut proxy_options = ProxyOptions::new();
        proxy_options.auto();
        return Some(proxy_options);
    }

    None
}

fn resolve_remote_redirect_policy() -> RemoteRedirect {
    let Some(policy) = read_non_empty_env(ENV_TRANSPORT_REMOTE_REDIRECT) else {
        return RemoteRedirect::Initial;
    };
    if policy.eq_ignore_ascii_case("none") {
        return RemoteRedirect::None;
    }
    if policy.eq_ignore_ascii_case("all") {
        return RemoteRedirect::All;
    }

    RemoteRedirect::Initial
}

fn read_non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
