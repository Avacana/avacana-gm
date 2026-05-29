//! SSH/HTTPS auth providers for `GitManager`.

mod git_config;
mod git_credentials;
mod interactive;
mod netrc;
mod os_store;
mod ssh_agent;
mod ssh_config;
mod ssh_identity;
mod url;
mod username_hint;

pub use git_config::GitConfigCredentialProvider;
pub use git_credentials::GitCredentialsFileProvider;
pub use interactive::InteractiveCallbackProvider;
pub use netrc::NetrcProvider;
pub use os_store::{OsSecretStoreProvider, OsStoreEntry};
pub use ssh_agent::SshAgentProvider;
pub use ssh_config::{SshConfigProvider, SshConfigResolved};
pub use ssh_identity::{SshIdentityFileProvider, SshIdentitySource};
pub use url::UrlCredentialProvider;
pub use username_hint::UsernameHintProvider;
