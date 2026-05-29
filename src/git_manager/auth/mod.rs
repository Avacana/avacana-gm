//! Core of the `GitManager` auth layer.

mod attempt_budget;
mod capabilities;
mod chain;
mod context;
mod environment_probe;
mod material;
pub mod providers;

pub use attempt_budget::AuthAttemptBudget;
pub use capabilities::{AuthCapabilities, AuthEnvironmentMode};
pub use chain::{
    AuthAttemptOutcome, AuthChain, AuthError, AuthErrorCode, AuthErrorDiagnostic, AuthResult,
    GitAuthProvider,
};
pub use context::{AuthContext, AuthTransport};
pub use environment_probe::{
    build_default_auth_chain_for_environment,
    build_default_auth_chain_for_environment_with_diagnostics,
    build_default_auth_chain_for_profile, AuthEnvironmentModeSource, AuthEnvironmentProbe,
    AuthEnvironmentProbeDiagnostics, AuthEnvironmentProbeSnapshot, AuthEnvironmentProbeWarning,
    AuthEnvironmentProbeWarningCode, AuthEnvironmentProfile,
};
pub use material::{
    AuthMaterial, AuthMaterialKind, CredentialLookupKey, HttpsAuthMaterial, NetrcEntry,
    SshCredentialSource,
};
pub use providers::{
    GitConfigCredentialProvider, GitCredentialsFileProvider, InteractiveCallbackProvider,
    NetrcProvider, OsSecretStoreProvider, OsStoreEntry, SshAgentProvider, SshConfigProvider,
    SshConfigResolved, SshIdentityFileProvider, SshIdentitySource, UrlCredentialProvider,
    UsernameHintProvider,
};
