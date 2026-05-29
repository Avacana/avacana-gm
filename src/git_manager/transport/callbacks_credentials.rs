use super::{auth_error_to_git2_error, non_empty, CallbackRuntimeHandle};
use crate::git_manager::auth::{
    AuthAttemptBudget, AuthAttemptOutcome, AuthChain, AuthContext, AuthError, AuthErrorCode,
    AuthMaterial, AuthMaterialKind, SshCredentialSource,
};
use crate::git_manager::transport::redact_url_userinfo;
use git2::{Cred, CredentialType, Error, RemoteCallbacks};
use std::path::Path;

const DEFAULT_TOKEN_USERNAME: &str = "oauth2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SshCredentialStrategy {
    Agent,
    KeyFile,
    UsernameFallback,
}

pub(crate) fn configure_credentials_callback<'callbacks>(
    callbacks: &mut RemoteCallbacks<'callbacks>,
    auth_chain: &'callbacks AuthChain,
    auth_context: AuthContext,
    auth_budget: &'callbacks mut AuthAttemptBudget,
    runtime_state: CallbackRuntimeHandle,
) {
    callbacks.credentials(move |callback_url, username_from_url, allowed_types| {
        resolve_credential(
            auth_chain,
            &auth_context,
            auth_budget,
            &runtime_state,
            callback_url,
            username_from_url,
            allowed_types,
        )
    });
}

fn resolve_credential(
    auth_chain: &AuthChain,
    auth_context: &AuthContext,
    auth_budget: &mut AuthAttemptBudget,
    runtime_state: &CallbackRuntimeHandle,
    callback_url: &str,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> Result<Cred, Error> {
    runtime_state.increment_credential_callbacks();
    let callback_context = build_callback_context(auth_context, username_from_url);
    let mut selected_credential: Option<Cred> = None;

    let auth_result = auth_chain.authenticate(&callback_context, auth_budget, |material| {
        try_build_credential_from_material(material, username_from_url, allowed_types).map_or_else(
            || {
                tracing::trace!(
                    callback_url = %redact_url_userinfo(callback_url),
                    allowed_types = ?allowed_types,
                    credential_source_kind = %material.credential_source_kind(),
                    material = %material.redacted_label(),
                    "auth material does not match requested git2 credential type"
                );
                AuthAttemptOutcome::RetryableError
            },
            |credential| {
                selected_credential = Some(credential);
                AuthAttemptOutcome::Authenticated
            },
        )
    });

    match auth_result {
        Ok(auth_material) => {
            runtime_state.record_material(&auth_material);
            selected_credential.ok_or_else(|| {
                auth_error_to_git2_error(&AuthError::new(
                    AuthErrorCode::NoCredentials,
                    "auth chain returned material but git2 credential conversion failed",
                ))
            })
        }
        Err(auth_error) if auth_error.code() == AuthErrorCode::NoCredentials => {
            tracing::trace!(
                callback_url = %redact_url_userinfo(callback_url),
                "auth chain exhausted without transport-side credential helper bypass"
            );
            Err(auth_error_to_git2_error(&auth_error))
        }
        Err(auth_error) => Err(auth_error_to_git2_error(&auth_error)),
    }
}

fn build_callback_context(
    auth_context: &AuthContext,
    username_from_url: Option<&str>,
) -> AuthContext {
    username_from_url.and_then(non_empty).map_or_else(
        || auth_context.clone(),
        |username| auth_context.clone().with_username_hint(username),
    )
}

pub(crate) fn try_build_credential_from_material(
    material: &AuthMaterial,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> Option<Cred> {
    match material.kind() {
        AuthMaterialKind::UsernamePassword => {
            build_username_password_credential(material, username_from_url, allowed_types)
        }
        AuthMaterialKind::Token => {
            build_token_credential(material, username_from_url, allowed_types)
        }
        AuthMaterialKind::UsernameOnly => {
            build_username_credential(material, username_from_url, allowed_types)
        }
        AuthMaterialKind::SshKey(ssh_source) => {
            build_ssh_key_credential(material, ssh_source, username_from_url, allowed_types)
        }
    }
}

fn build_username_password_credential(
    material: &AuthMaterial,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> Option<Cred> {
    if !allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
        return None;
    }

    let username = material.principal().or(username_from_url)?;
    let secret = material.secret()?;
    map_cred_result(
        Cred::userpass_plaintext(username, secret),
        material,
        "userpass_plaintext",
    )
}

fn build_token_credential(
    material: &AuthMaterial,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> Option<Cred> {
    if !allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
        return None;
    }

    let username = material
        .principal()
        .or(username_from_url)
        .unwrap_or(DEFAULT_TOKEN_USERNAME);
    let secret = material.secret()?;
    map_cred_result(
        Cred::userpass_plaintext(username, secret),
        material,
        "token_userpass",
    )
}

fn build_username_credential(
    material: &AuthMaterial,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> Option<Cred> {
    if !allowed_types.contains(CredentialType::USERNAME) {
        return None;
    }

    let username = material.principal().or(username_from_url)?;
    map_cred_result(Cred::username(username), material, "username")
}

fn build_ssh_key_credential(
    material: &AuthMaterial,
    ssh_source: SshCredentialSource,
    username_from_url: Option<&str>,
    allowed_types: CredentialType,
) -> Option<Cred> {
    let username = material.principal().or(username_from_url)?;

    match select_ssh_credential_strategy(ssh_source, allowed_types)? {
        SshCredentialStrategy::Agent => map_cred_result(
            Cred::ssh_key_from_agent(username),
            material,
            "ssh_key_from_agent",
        ),
        SshCredentialStrategy::KeyFile => {
            let private_key_path = Path::new(material.secret()?);
            map_cred_result(
                Cred::ssh_key(username, None, private_key_path, None),
                material,
                "ssh_key_file",
            )
        }
        SshCredentialStrategy::UsernameFallback => {
            map_cred_result(Cred::username(username), material, "username")
        }
    }
}

pub(crate) const fn select_ssh_credential_strategy(
    ssh_source: SshCredentialSource,
    allowed_types: CredentialType,
) -> Option<SshCredentialStrategy> {
    if allowed_types.contains(CredentialType::SSH_KEY) {
        return Some(match ssh_source {
            SshCredentialSource::Agent => SshCredentialStrategy::Agent,
            SshCredentialSource::KeyFile => SshCredentialStrategy::KeyFile,
        });
    }

    if allowed_types.contains(CredentialType::USERNAME) {
        return Some(SshCredentialStrategy::UsernameFallback);
    }

    None
}

fn map_cred_result(
    credential_result: Result<Cred, Error>,
    material: &AuthMaterial,
    strategy: &'static str,
) -> Option<Cred> {
    credential_result.map_or_else(
        |error| {
            tracing::trace!(
                credential_source_kind = %material.credential_source_kind(),
                material = %material.redacted_label(),
                strategy = strategy,
                error = %error,
                "git2 credential conversion failed"
            );
            None
        },
        Some,
    )
}
