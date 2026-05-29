use std::fmt;

/// Typed source of SSH credential material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SshCredentialSource {
    /// The SSH credential must be requested via the agent (`ssh-agent`).
    Agent,
    /// The SSH credential must be built from a local key-file path.
    KeyFile,
}

impl SshCredentialSource {
    /// Returns the stable machine-readable code of the SSH credential source.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "ssh_agent",
            Self::KeyFile => "ssh_key_file",
        }
    }
}

impl fmt::Display for SshCredentialSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Kind of credential material that an auth provider may return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthMaterialKind {
    /// SSH key with an explicit typed source (`ssh-agent` or key-file).
    SshKey(SshCredentialSource),
    /// A username/password pair.
    UsernamePassword,
    /// Token-based credential.
    Token,
    /// Username only (no secret).
    UsernameOnly,
}

impl AuthMaterialKind {
    /// Returns the stable machine-readable code of the material kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SshKey(_) => "ssh_key",
            Self::UsernamePassword => "username_password",
            Self::Token => "token",
            Self::UsernameOnly => "username_only",
        }
    }

    /// Returns the machine-readable code of the typed credential source.
    #[must_use]
    pub const fn credential_source_kind(self) -> &'static str {
        match self {
            Self::SshKey(source) => source.as_str(),
            Self::UsernamePassword => "username_password",
            Self::Token => "token",
            Self::UsernameOnly => "username_only",
        }
    }

    /// Returns the typed SSH source if the material is SSH-related.
    #[must_use]
    pub const fn ssh_credential_source(self) -> Option<SshCredentialSource> {
        match self {
            Self::SshKey(source) => Some(source),
            Self::UsernamePassword | Self::Token | Self::UsernameOnly => None,
        }
    }
}

impl fmt::Display for AuthMaterialKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
