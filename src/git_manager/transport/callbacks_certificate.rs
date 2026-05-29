use super::auth_error_to_git2_error;
use crate::git_manager::auth::{AuthError, AuthErrorCode, AuthResult};
use crate::git_manager::transport::certificates::{HostKeyPolicyResult, KnownHostsVerifier};
use base64::Engine as _;
use git2::cert::Cert;
use git2::{CertificateCheckStatus, Error, RemoteCallbacks};

const DEFAULT_SSH_PORT: u16 = 22;

pub(crate) fn configure_certificate_callback(
    callbacks: &mut RemoteCallbacks<'_>,
    known_hosts_verifier: KnownHostsVerifier,
) {
    callbacks.certificate_check(move |certificate, callback_host| {
        verify_git2_certificate(&known_hosts_verifier, certificate, callback_host)
    });
}

fn verify_git2_certificate(
    known_hosts_verifier: &KnownHostsVerifier,
    certificate: &Cert<'_>,
    callback_host: &str,
) -> Result<CertificateCheckStatus, Error> {
    let Some(host_key_certificate) = certificate.as_hostkey() else {
        return Ok(CertificateCheckStatus::CertificatePassthrough);
    };

    let Some(host_key_blob) = host_key_certificate.hostkey() else {
        let auth_error = AuthError::new(
            AuthErrorCode::HostKeyMismatch,
            format!("ssh host key for `{callback_host}` does not expose raw key bytes"),
        );
        return Err(auth_error_to_git2_error(&auth_error));
    };

    let host_key_type = host_key_certificate
        .hostkey_type()
        .map_or("unknown", |host_key_type| host_key_type.name());

    verify_ssh_host_key_payload(
        known_hosts_verifier,
        callback_host,
        host_key_type,
        host_key_blob,
    )
    .map(|_| CertificateCheckStatus::CertificateOk)
    .map_err(|auth_error| auth_error_to_git2_error(&auth_error))
}

pub(crate) fn verify_ssh_host_key_payload(
    known_hosts_verifier: &KnownHostsVerifier,
    callback_host: &str,
    key_type: &str,
    key_blob: &[u8],
) -> AuthResult<HostKeyPolicyResult> {
    if key_blob.is_empty() {
        return Err(AuthError::new(
            AuthErrorCode::HostKeyMismatch,
            format!("ssh host key for `{callback_host}` is empty"),
        ));
    }

    let parsed_host = parse_callback_host(callback_host);
    if parsed_host.host.is_empty() {
        return Err(AuthError::new(
            AuthErrorCode::HostKeyMismatch,
            "ssh callback host is empty and cannot be verified against known_hosts",
        ));
    }

    let host_key_blob_base64 = base64::engine::general_purpose::STANDARD.encode(key_blob);
    known_hosts_verifier.verify_host_key(
        &parsed_host.host,
        parsed_host.port,
        key_type,
        &host_key_blob_base64,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedCallbackHost {
    pub(crate) host: String,
    pub(crate) port: u16,
}

pub(crate) fn parse_callback_host(callback_host: &str) -> ParsedCallbackHost {
    let callback_host = strip_username_prefix(callback_host.trim());

    if let Some((host, port)) = parse_bracketed_host_with_port(callback_host) {
        return ParsedCallbackHost {
            host: host.to_string(),
            port,
        };
    }

    if let Some((host, port)) = parse_host_with_port(callback_host) {
        return ParsedCallbackHost {
            host: host.to_string(),
            port,
        };
    }

    ParsedCallbackHost {
        host: strip_brackets(callback_host).to_string(),
        port: DEFAULT_SSH_PORT,
    }
}

fn parse_bracketed_host_with_port(value: &str) -> Option<(&str, u16)> {
    if !value.starts_with('[') {
        return None;
    }

    let closing_bracket = value.find("]:")?;
    let host = &value[1..closing_bracket];
    let raw_port = &value[(closing_bracket + 2)..];
    let port = raw_port.parse::<u16>().ok()?;
    Some((host, port))
}

fn parse_host_with_port(value: &str) -> Option<(&str, u16)> {
    let (host, raw_port) = value.rsplit_once(':')?;
    if host.is_empty() || host.contains(':') {
        return None;
    }

    let port = raw_port.parse::<u16>().ok()?;
    Some((host, port))
}

fn strip_username_prefix(value: &str) -> &str {
    value.rsplit_once('@').map_or(value, |(_, host)| host)
}

fn strip_brackets(value: &str) -> &str {
    value
        .strip_prefix('[')
        .and_then(|without_prefix| without_prefix.strip_suffix(']'))
        .unwrap_or(value)
}
