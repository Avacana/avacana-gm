use std::collections::{HashMap, HashSet};
use std::fmt;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

/// Default wall-clock timeout for the entire auth flow.
pub const DEFAULT_AUTH_TOTAL_TIMEOUT: Duration = Duration::from_secs(30);
/// Default upper bound on attempts per operation.
pub const DEFAULT_MAX_AUTH_ATTEMPTS: usize = 6;
/// Default attempt limit for a single fingerprint.
pub const DEFAULT_MAX_ATTEMPTS_PER_MATERIAL: usize = 1;

/// Reason the budget policy rejected a new auth attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthAttemptBudgetRejection {
    /// The overall deadline for the auth operation has been exceeded.
    DeadlineExceeded,
    /// The overall attempt limit has been exceeded.
    TotalAttemptsExhausted,
    /// The attempt limit for a specific material has been exceeded.
    MaterialAttemptsExhausted,
    /// The material was previously marked invalid and cannot be reused.
    MaterialInvalidated,
}

impl AuthAttemptBudgetRejection {
    /// Returns the machine-readable rejection code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::TotalAttemptsExhausted => "total_attempts_exhausted",
            Self::MaterialAttemptsExhausted => "material_attempts_exhausted",
            Self::MaterialInvalidated => "material_invalidated",
        }
    }
}

impl fmt::Display for AuthAttemptBudgetRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Auth-flow attempt budget: bounds the number of attempts and wall-clock time.
#[derive(Debug, Clone)]
pub struct AuthAttemptBudget {
    max_attempts: usize,
    per_material_attempt_limit: usize,
    deadline: Instant,
    total_attempts: usize,
    attempts_by_material: HashMap<String, usize>,
    invalidated_materials: HashSet<String>,
}

impl AuthAttemptBudget {
    /// Creates a budget with a timeout measured from the current instant.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        max_attempts: NonZeroUsize,
        per_material_attempt_limit: NonZeroUsize,
        auth_total_timeout: Duration,
    ) -> Self {
        let now = Instant::now();
        let deadline = now.checked_add(auth_total_timeout).unwrap_or(now);
        Self::with_deadline(max_attempts, per_material_attempt_limit, deadline)
    }

    /// Creates a budget with a fixed wall-clock deadline.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_deadline(
        max_attempts: NonZeroUsize,
        per_material_attempt_limit: NonZeroUsize,
        deadline: Instant,
    ) -> Self {
        Self {
            max_attempts: max_attempts.get(),
            per_material_attempt_limit: per_material_attempt_limit.get(),
            deadline,
            total_attempts: 0,
            attempts_by_material: HashMap::new(),
            invalidated_materials: HashSet::new(),
        }
    }

    /// Attempts to reserve a single attempt for the given fingerprint.
    ///
    /// # Errors
    /// Returns the rejection reason if the budget does not allow the attempt to start.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn try_start_attempt(
        &mut self,
        material_fingerprint: &str,
        now: Instant,
    ) -> Result<(), AuthAttemptBudgetRejection> {
        if self.has_timed_out(now) {
            return Err(AuthAttemptBudgetRejection::DeadlineExceeded);
        }
        if self.invalidated_materials.contains(material_fingerprint) {
            return Err(AuthAttemptBudgetRejection::MaterialInvalidated);
        }
        if self.total_attempts >= self.max_attempts {
            return Err(AuthAttemptBudgetRejection::TotalAttemptsExhausted);
        }

        let material_attempts = self
            .attempts_by_material
            .get(material_fingerprint)
            .copied()
            .unwrap_or(0);
        if material_attempts >= self.per_material_attempt_limit {
            return Err(AuthAttemptBudgetRejection::MaterialAttemptsExhausted);
        }

        self.total_attempts += 1;
        self.attempts_by_material
            .insert(material_fingerprint.to_string(), material_attempts + 1);
        Ok(())
    }

    /// Attempts to reserve an attempt measured from the current instant.
    ///
    /// # Errors
    /// Returns the rejection reason if the budget does not allow the attempt to start.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn try_start_attempt_now(
        &mut self,
        material_fingerprint: &str,
    ) -> Result<(), AuthAttemptBudgetRejection> {
        self.try_start_attempt(material_fingerprint, Instant::now())
    }

    /// Marks a credential fingerprint as invalid for the duration of the current operation.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn invalidate(&mut self, material_fingerprint: &str) {
        self.invalidated_materials
            .insert(material_fingerprint.to_string());
    }

    /// Returns `true` if the credential fingerprint has already been marked invalid.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn is_invalidated(&self, material_fingerprint: &str) -> bool {
        self.invalidated_materials.contains(material_fingerprint)
    }

    /// Returns `true` if the wall-clock deadline has already passed.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn has_timed_out(&self, now: Instant) -> bool {
        now >= self.deadline
    }

    /// Returns `true` if the wall-clock deadline has already passed as of now.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn has_timed_out_now(&self) -> bool {
        self.has_timed_out(Instant::now())
    }

    /// Returns the total number of attempts already consumed.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn total_attempts(&self) -> usize {
        self.total_attempts
    }

    /// Returns the overall attempt limit.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    /// Returns the attempt limit per credential fingerprint.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn per_material_attempt_limit(&self) -> usize {
        self.per_material_attempt_limit
    }

    /// Returns the number of remaining attempts.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn remaining_attempts(&self) -> usize {
        self.max_attempts.saturating_sub(self.total_attempts)
    }
}

impl Default for AuthAttemptBudget {
    fn default() -> Self {
        let max_attempts = NonZeroUsize::new(DEFAULT_MAX_AUTH_ATTEMPTS)
            .expect("default max attempts must be non-zero");
        let per_material_attempt_limit = NonZeroUsize::new(DEFAULT_MAX_ATTEMPTS_PER_MATERIAL)
            .expect("default per-material limit must be non-zero");
        Self::new(
            max_attempts,
            per_material_attempt_limit,
            DEFAULT_AUTH_TOTAL_TIMEOUT,
        )
    }
}

