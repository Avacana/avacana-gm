use super::CallbackRuntimeHandle;
use git2::RemoteCallbacks;

const SIDEBAND_MESSAGE_MAX_CHARS: usize = 160;

pub(crate) fn configure_progress_callbacks(
    callbacks: &mut RemoteCallbacks<'_>,
    runtime_state: CallbackRuntimeHandle,
) {
    callbacks.transfer_progress(|progress| {
        tracing::trace!(
            received_objects = progress.received_objects(),
            total_objects = progress.total_objects(),
            indexed_objects = progress.indexed_objects(),
            received_bytes = progress.received_bytes(),
            "git2 transfer progress event"
        );
        true
    });

    let sideband_runtime = runtime_state.clone();
    callbacks.sideband_progress(move |payload| {
        let sideband_message = format_sideband_message(payload);
        tracing::trace!(sideband = %sideband_message, "git2 sideband progress event");
        sideband_runtime.push_transport_message(sideband_message);
        true
    });

    let update_tips_runtime = runtime_state.clone();
    callbacks.update_tips(move |reference, previous_oid, new_oid| {
        update_tips_runtime.push_update_tip(
            reference.to_string(),
            previous_oid.to_string(),
            new_oid.to_string(),
        );
        tracing::trace!(
            reference = reference,
            previous_oid = %previous_oid,
            new_oid = %new_oid,
            "git2 update_tips callback event"
        );
        true
    });

    callbacks.push_negotiation(move |updates| {
        for update in updates {
            runtime_state.push_negotiated_update(
                update.src_refname().map(std::string::ToString::to_string),
                update.dst_refname().map(std::string::ToString::to_string),
                update.src().to_string(),
                update.dst().to_string(),
            );
        }
        Ok(())
    });
}

fn format_sideband_message(payload: &[u8]) -> String {
    let message = std::str::from_utf8(payload).map_or("<non-utf8-sideband>", str::trim);
    shorten_for_trace(message, SIDEBAND_MESSAGE_MAX_CHARS)
}

fn shorten_for_trace(message: &str, max_chars: usize) -> String {
    let mut chars = message.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}
