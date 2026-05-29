#![allow(clippy::redundant_pub_crate)]

use super::certificates::KnownHostsVerifier;
use crate::git_manager::auth::{AuthAttemptBudget, AuthChain, AuthContext, AuthError};
use git2::{Error, RemoteCallbacks};
use std::cell::RefCell;
use std::rc::Rc;

#[path = "callbacks_certificate.rs"]
mod certificate_callbacks;
#[path = "callbacks_credentials.rs"]
mod credentials_callbacks;
#[path = "callbacks_progress.rs"]
mod progress_callbacks;

use certificate_callbacks::configure_certificate_callback;
use credentials_callbacks::configure_credentials_callback;
use progress_callbacks::configure_progress_callbacks;

#[derive(Debug, Clone, Default)]
pub(super) struct CallbackRuntimeHandle {
    state: Rc<RefCell<CallbackRuntimeState>>,
}

#[derive(Debug, Default)]
struct CallbackRuntimeState {
    credential_callback_count: usize,
    last_used_material_fingerprint: Option<String>,
    last_used_material_label: Option<String>,
    transport_messages: Vec<String>,
    update_tips: Vec<(String, String, String)>,
    push_updates: Vec<(Option<String>, Option<String>, String, String)>,
}

impl CallbackRuntimeHandle {
    fn increment_credential_callbacks(&self) {
        let mut state = self.state.borrow_mut();
        state.credential_callback_count += 1;
    }

    fn record_material(&self, material: &crate::git_manager::auth::AuthMaterial) {
        let mut state = self.state.borrow_mut();
        state.last_used_material_fingerprint = Some(material.fingerprint().to_string());
        state.last_used_material_label = Some(material.redacted_label());
    }

    pub(super) fn last_used_material_fingerprint(&self) -> Option<String> {
        self.state.borrow().last_used_material_fingerprint.clone()
    }

    pub(super) fn credential_callback_count(&self) -> usize {
        self.state.borrow().credential_callback_count
    }

    pub(super) fn transport_messages(&self) -> Vec<String> {
        self.state.borrow().transport_messages.clone()
    }

    pub(super) fn update_tips(&self) -> Vec<(String, String, String)> {
        self.state.borrow().update_tips.clone()
    }

    pub(super) fn push_updates(&self) -> Vec<(Option<String>, Option<String>, String, String)> {
        self.state.borrow().push_updates.clone()
    }

    fn push_transport_message(&self, message: String) {
        self.state.borrow_mut().transport_messages.push(message);
    }

    fn push_update_tip(&self, reference: String, previous_oid: String, new_oid: String) {
        self.state
            .borrow_mut()
            .update_tips
            .push((reference, previous_oid, new_oid));
    }

    fn push_negotiated_update(
        &self,
        src_refname: Option<String>,
        dst_refname: Option<String>,
        src_oid: String,
        dst_oid: String,
    ) {
        self.state
            .borrow_mut()
            .push_updates
            .push((src_refname, dst_refname, src_oid, dst_oid));
    }
}

pub(super) fn configure_git2_remote_callbacks<'callbacks>(
    callbacks: &mut RemoteCallbacks<'callbacks>,
    auth_chain: &'callbacks AuthChain,
    auth_context: &AuthContext,
    auth_budget: &'callbacks mut AuthAttemptBudget,
    known_hosts_verifier: KnownHostsVerifier,
) -> CallbackRuntimeHandle {
    let runtime_state = CallbackRuntimeHandle::default();

    configure_credentials_callback(
        callbacks,
        auth_chain,
        auth_context.clone(),
        auth_budget,
        runtime_state.clone(),
    );
    configure_certificate_callback(callbacks, known_hosts_verifier);
    configure_progress_callbacks(callbacks, runtime_state.clone());

    runtime_state
}

fn auth_error_to_git2_error(auth_error: &AuthError) -> Error {
    Error::from_str(&auth_error.to_string())
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}
