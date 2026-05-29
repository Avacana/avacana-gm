pub(super) use super::attempt_budget::{AuthAttemptBudget, AuthAttemptBudgetRejection};
pub(super) use super::capabilities::AuthCapabilities;
pub(super) use super::context::AuthContext;
pub(super) use super::material::AuthMaterial;

#[path = "chain_contracts.rs"]
mod contracts;
#[path = "chain_execution.rs"]
mod execution;

pub use contracts::{
    AuthAttemptOutcome, AuthError, AuthErrorCode, AuthErrorDiagnostic, AuthResult, GitAuthProvider,
};
pub use execution::AuthChain;
