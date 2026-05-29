//! Diagnostics layer of the `GitManager` framework.

/// Base diagnostic record for `GitManager` tracing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiagnosticRecord {
    pub operation: &'static str,
    pub detail: String,
}
