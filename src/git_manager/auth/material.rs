#[path = "material_auth.rs"]
mod auth_material;
#[path = "material_https.rs"]
mod https_auth_material;
#[path = "material_lookup.rs"]
mod lookup_key;
#[path = "material_netrc.rs"]
mod netrc_entry;
#[path = "material_redaction.rs"]
mod redaction;
#[path = "material_types.rs"]
mod types;

pub use auth_material::AuthMaterial;
pub use https_auth_material::HttpsAuthMaterial;
pub use lookup_key::CredentialLookupKey;
pub use netrc_entry::NetrcEntry;
pub use types::{AuthMaterialKind, SshCredentialSource};
