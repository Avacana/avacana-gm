pub(super) use super::capabilities::{AuthCapabilities, AuthEnvironmentMode};
use super::chain::AuthChain;
use super::providers::{
    GitConfigCredentialProvider, GitCredentialsFileProvider, InteractiveCallbackProvider,
    NetrcProvider, OsSecretStoreProvider, SshAgentProvider, SshConfigProvider,
    SshIdentityFileProvider, UrlCredentialProvider, UsernameHintProvider,
};

#[path = "environment_probe_profile.rs"]
mod profile;

pub use profile::{
    AuthEnvironmentModeSource, AuthEnvironmentProbeDiagnostics, AuthEnvironmentProbeSnapshot,
    AuthEnvironmentProbeWarning, AuthEnvironmentProbeWarningCode, AuthEnvironmentProfile,
};

/// Detector for `GitAuth` modes based on OS/env signals.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuthEnvironmentProbe;

impl AuthEnvironmentProbe {
    /// Probes the current environment and returns the auth-policy profile.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn probe() -> AuthEnvironmentProfile {
        Self::probe_with_diagnostics().into_profile()
    }

    /// Probes the current environment and returns the profile together with typed diagnostics.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn probe_with_diagnostics() -> AuthEnvironmentProbeSnapshot {
        probe_with_reader_with_diagnostics(|name| std::env::var(name).ok())
    }
}

/// Builds the default auth chain and returns the environment profile used by the probe.
#[cfg_attr(
    all(debug_assertions, feature = "trace_logs"),
    tracing::instrument(skip_all)
)]
#[must_use]
pub fn build_default_auth_chain_for_environment() -> (AuthChain, AuthEnvironmentProfile) {
    let profile = AuthEnvironmentProbe::probe_with_diagnostics().into_profile();
    let chain = build_default_auth_chain_for_profile(&profile);
    (chain, profile)
}

/// Builds the default auth chain for the current environment and returns the typed probe diagnostics.
#[cfg_attr(
    all(debug_assertions, feature = "trace_logs"),
    tracing::instrument(skip_all)
)]
#[must_use]
pub fn build_default_auth_chain_for_environment_with_diagnostics() -> (
    AuthChain,
    AuthEnvironmentProfile,
    AuthEnvironmentProbeDiagnostics,
) {
    let snapshot = AuthEnvironmentProbe::probe_with_diagnostics();
    let diagnostics = snapshot.diagnostics().clone();
    let profile = snapshot.into_profile();
    let chain = build_default_auth_chain_for_profile(&profile);
    (chain, profile, diagnostics)
}

/// Builds the default auth chain for a precomputed profile.
#[cfg_attr(
    all(debug_assertions, feature = "trace_logs"),
    tracing::instrument(skip_all)
)]
#[must_use]
pub fn build_default_auth_chain_for_profile(profile: &AuthEnvironmentProfile) -> AuthChain {
    let mut auth_chain = AuthChain::new(profile.capabilities());

    let ssh_config_provider = SshConfigProvider::default();
    auth_chain.push_provider(Box::new(ssh_config_provider.clone()));
    auth_chain.push_provider(Box::new(SshAgentProvider::default()));
    auth_chain.push_provider(Box::new(SshIdentityFileProvider::with_default_paths(
        ssh_config_provider.clone(),
    )));
    auth_chain.push_provider(Box::new(UsernameHintProvider::new(ssh_config_provider)));

    let git_config_provider = GitConfigCredentialProvider::default();
    auth_chain.push_provider(Box::new(UrlCredentialProvider::new()));
    auth_chain.push_provider(Box::new(git_config_provider.clone()));
    auth_chain.push_provider(Box::new(OsSecretStoreProvider::with_environment(
        git_config_provider.clone(),
        profile.os_store_available(),
    )));
    auth_chain.push_provider(Box::new(GitCredentialsFileProvider::with_default_paths(
        git_config_provider.clone(),
    )));
    auth_chain.push_provider(Box::new(NetrcProvider::with_default_path(
        git_config_provider,
    )));
    auth_chain.push_provider(Box::new(
        InteractiveCallbackProvider::with_environment_opt_in(),
    ));

    auth_chain
}

fn probe_with_reader_with_diagnostics<F>(mut read_var: F) -> AuthEnvironmentProbeSnapshot
where
    F: FnMut(&str) -> Option<String>,
{
    let mut diagnostics = AuthEnvironmentProbeDiagnostics::default();
    let mode = resolve_environment_mode(&mut read_var, &mut diagnostics);
    let allow_ssh =
        parse_bool_env_with_diagnostics(&mut read_var, "AVACANA_GM_AUTH_ALLOW_SSH", &mut diagnostics)
            .unwrap_or(true);
    let allow_https = parse_bool_env_with_diagnostics(
        &mut read_var,
        "AVACANA_GM_AUTH_ALLOW_HTTPS",
        &mut diagnostics,
    )
    .unwrap_or(true);
    let allow_interactive = parse_bool_env_with_diagnostics(
        &mut read_var,
        "AVACANA_GM_AUTH_ALLOW_INTERACTIVE",
        &mut diagnostics,
    )
    .unwrap_or_else(|| default_interactive_allowed(mode, &mut read_var, &mut diagnostics));
    let os_store_available = detect_os_store_available(mode, &mut read_var, &mut diagnostics);
    let accept_new_host = parse_bool_env_with_diagnostics(
        &mut read_var,
        "AVACANA_GM_AUTH_ACCEPT_NEW_HOST",
        &mut diagnostics,
    )
    .unwrap_or(false);

    let profile = AuthEnvironmentProfile::from_mode(mode)
        .with_transport_support(allow_ssh, allow_https)
        .with_allow_interactive(allow_interactive)
        .with_os_store_available(os_store_available)
        .with_accept_new_host(accept_new_host);
    AuthEnvironmentProbeSnapshot::new(profile, diagnostics)
}

fn resolve_environment_mode<F>(
    read_var: &mut F,
    diagnostics: &mut AuthEnvironmentProbeDiagnostics,
) -> AuthEnvironmentMode
where
    F: FnMut(&str) -> Option<String>,
{
    if let Some(mode_from_env) = read_non_empty_env(read_var, "AVACANA_GM_AUTH_ENV_MODE") {
        if let Some(parsed_mode) = parse_environment_mode(mode_from_env.as_str()) {
            diagnostics.set_mode_source(AuthEnvironmentModeSource::ExplicitEnvironmentOverride);
            return parsed_mode;
        }
        diagnostics.push_warning(AuthEnvironmentProbeWarning::invalid_mode(
            "AVACANA_GM_AUTH_ENV_MODE",
            mode_from_env,
        ));
    }

    if parse_bool_env_with_diagnostics(read_var, "AVACANA_GM_AUTH_RESTRICTED", diagnostics)
        .is_some_and(|value| value)
    {
        diagnostics.set_mode_source(AuthEnvironmentModeSource::RestrictedFlag);
        return AuthEnvironmentMode::RestrictedSandbox;
    }
    if parse_bool_env_with_diagnostics(read_var, "CI", diagnostics).is_some_and(|value| value)
        || parse_bool_env_with_diagnostics(read_var, "AVACANA_GM_AUTH_HEADLESS", diagnostics)
            .is_some_and(|value| value)
    {
        diagnostics.set_mode_source(AuthEnvironmentModeSource::HeadlessSignal);
        return AuthEnvironmentMode::HeadlessCi;
    }

    diagnostics.set_mode_source(AuthEnvironmentModeSource::DesktopDefault);
    AuthEnvironmentMode::DesktopFull
}

fn default_interactive_allowed<F>(
    mode: AuthEnvironmentMode,
    read_var: &mut F,
    diagnostics: &mut AuthEnvironmentProbeDiagnostics,
) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    if !matches!(mode, AuthEnvironmentMode::DesktopFull) {
        return false;
    }
    parse_bool_env_with_diagnostics(read_var, "GIT_TERMINAL_PROMPT", diagnostics)
        .is_none_or(|value| value)
}

fn detect_os_store_available<F>(
    mode: AuthEnvironmentMode,
    read_var: &mut F,
    diagnostics: &mut AuthEnvironmentProbeDiagnostics,
) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    if let Some(override_value) =
        parse_bool_env_with_diagnostics(read_var, "AVACANA_GM_AUTH_OS_STORE_AVAILABLE", diagnostics)
    {
        return override_value;
    }

    if !matches!(mode, AuthEnvironmentMode::DesktopFull) {
        return false;
    }

    #[cfg(target_os = "linux")]
    {
        read_non_empty_env(read_var, "DBUS_SESSION_BUS_ADDRESS").is_some()
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = read_var;
        true
    }
}

fn parse_environment_mode(value: &str) -> Option<AuthEnvironmentMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "desktop_full" | "desktop" => Some(AuthEnvironmentMode::DesktopFull),
        "headless_ci" | "headless" | "ci" => Some(AuthEnvironmentMode::HeadlessCi),
        "restricted_sandbox" | "restricted" | "sandbox" => {
            Some(AuthEnvironmentMode::RestrictedSandbox)
        }
        _ => None,
    }
}

fn parse_bool_env_with_diagnostics<F>(
    read_var: &mut F,
    name: &'static str,
    diagnostics: &mut AuthEnvironmentProbeDiagnostics,
) -> Option<bool>
where
    F: FnMut(&str) -> Option<String>,
{
    let raw = read_non_empty_env(read_var, name)?;
    let parsed = parse_bool(raw.as_str());
    if parsed.is_none() {
        diagnostics.push_warning(AuthEnvironmentProbeWarning::invalid_boolean(name, raw));
    }
    parsed
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn read_non_empty_env<F>(read_var: &mut F, name: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let value = read_var(name)?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
