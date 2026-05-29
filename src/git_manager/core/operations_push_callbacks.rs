use super::callback_errors::{fallback_rejected_ref_message, is_rejected_refs_error};
use crate::git_manager::core::operations_remote_transport_support::apply_push_network_options;
use git2::{PushOptions, RemoteCallbacks};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct PushCallbackOutcome {
    pub(super) updated_refs: Vec<String>,
    pub(super) rejected_refs: Vec<String>,
}

pub(super) fn execute_push_with_callbacks(
    remote: &mut git2::Remote<'_>,
    callbacks: &mut RemoteCallbacks<'_>,
    refspecs: &[String],
) -> Result<PushCallbackOutcome, git2::Error> {
    let callback_outcome = Rc::new(RefCell::new(PushCallbackOutcome::default()));
    let mut owned_callbacks = take_remote_callbacks(callbacks);
    register_push_update_reference_callback(&mut owned_callbacks, callback_outcome.clone());
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(owned_callbacks);
    apply_push_network_options(&mut push_options);
    let refspec_refs: Vec<&str> = refspecs.iter().map(String::as_str).collect();
    let push_result = remote.push(refspec_refs.as_slice(), Some(&mut push_options));
    let outcome = callback_outcome.borrow().clone();
    match push_result {
        Ok(()) => Ok(outcome),
        Err(_error) if !outcome.rejected_refs.is_empty() => Ok(outcome),
        Err(error) if is_rejected_refs_error(&error) => {
            let mut rejected_outcome = outcome;
            rejected_outcome
                .rejected_refs
                .push(fallback_rejected_ref_message(&error));
            Ok(rejected_outcome)
        }
        Err(error) => Err(error),
    }
}

fn register_push_update_reference_callback(
    callbacks: &mut RemoteCallbacks<'_>,
    callback_outcome: Rc<RefCell<PushCallbackOutcome>>,
) {
    callbacks.push_update_reference(move |reference_name, status| {
        let normalized_reference = normalize_reference_name(reference_name);
        if let Some(status_message) = status.and_then(super::non_empty) {
            callback_outcome
                .borrow_mut()
                .rejected_refs
                .push(format!("{normalized_reference}: {status_message}"));
        } else {
            callback_outcome
                .borrow_mut()
                .updated_refs
                .push(normalized_reference);
        }
        Ok(())
    });
}

fn normalize_reference_name(reference_name: &str) -> String {
    super::non_empty(reference_name).map_or_else(|| "<unknown-ref>".to_string(), str::to_owned)
}

fn take_remote_callbacks<'callbacks>(
    callbacks: &mut RemoteCallbacks<'callbacks>,
) -> RemoteCallbacks<'callbacks> {
    let mut owned_callbacks = RemoteCallbacks::new();
    std::mem::swap(&mut owned_callbacks, callbacks);
    owned_callbacks
}
