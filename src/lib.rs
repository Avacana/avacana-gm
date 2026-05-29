//! # avacana-gm
//!
//! Standalone, embeddable Git operations powered by [`libgit2`] through the [`git2`]
//! crate. `avacana-gm` replaces shelling out to the `git` command-line tool with native,
//! in-process operations while preserving command-line Git semantics.
//!
//! The entire public surface is re-exported at the crate root, so every type, trait, and
//! function documented below is reachable directly as `avacana_gm::*`.
//!
//! ## Highlights
//!
//! - **Full operation surface** — clone, fetch, pull, push, stage, commit, branch, switch,
//!   merge, rebase, cherry-pick, revert, stash, diff, status, refs, notes, and low-level
//!   plumbing.
//! - **No subprocesses** — every operation runs through `git2`/libgit2; the `git` binary is
//!   never spawned.
//! - **Layered authentication** — an ordered chain of credential providers (SSH agent, SSH
//!   config, SSH identity files, URL credentials, git config, OS keychain, git-credentials
//!   files, netrc, and an opt-in interactive provider) with an attempt budget.
//! - **Thread safety** — the whole public API is `Send + Sync`; a two-level lock (process-local
//!   plus filesystem) serializes mutating access per repository.
//! - **Async variants** — every operation has an `*_async` counterpart that runs the blocking
//!   work on [`tokio::task::spawn_blocking`].
//! - **Automatic retries** — transient network failures are retried with exponential backoff.
//! - **Typed request/result API** — each operation takes a dedicated `*Request` and returns a
//!   `GitResult<*Result>`, so the contract is self-documenting and forward-compatible.
//! - **Optional structured logging** — enable the [`trace_logs`](#cargo-features) feature to
//!   instrument the public methods with [`tracing`](https://docs.rs/tracing).
//!
//! ## Quick start
//!
//! ### Constructing a manager
//!
//! [`GitManagerFacade`] is the concrete implementation of the [`GitManager`] trait. The
//! default construction wires up the full production authentication and transport stack:
//!
//! ```rust,no_run
//! use avacana_gm::{GitManager, GitManagerFacade};
//!
//! // Production stack with the full authentication chain.
//! let manager = GitManagerFacade::default();
//!
//! // Equivalent, but through the explicit dependency-injection container (recommended when
//! // you want to customize or inspect the wiring).
//! use avacana_gm::GitManagerComponents;
//! let manager = GitManagerFacade::new(GitManagerComponents::production());
//! # let _ = manager;
//! ```
//!
//! ### Cloning a repository
//!
//! ```rust,no_run
//! use avacana_gm::{GitManager, GitManagerFacade, CloneRequest, FetchTagMode};
//! use std::path::PathBuf;
//!
//! let manager = GitManagerFacade::default();
//! let request = CloneRequest {
//!     repository_url: "https://github.com/user/repo.git".to_string(),
//!     destination_path: PathBuf::from("/tmp/my-clone"),
//!     branch: Some("main".to_string()),
//!     depth: None, // full clone; use `Some(1)` for a shallow clone
//!     tag_mode: FetchTagMode::Auto,
//!     mirror: false,
//! };
//!
//! let result = manager.clone_repository(&request)?;
//! println!("cloned into {:?}", result.repository_path);
//! println!("checked out branch: {:?}", result.checked_out_branch);
//! # Ok::<(), avacana_gm::GitError>(())
//! ```
//!
//! ### Stage, commit, and push
//!
//! ```rust,no_run
//! use avacana_gm::{
//!     GitManager, GitManagerFacade,
//!     StageRequest, CommitRequest, PushRequest,
//!     EmptyCommitPolicy, HooksPolicy,
//! };
//! use std::path::PathBuf;
//!
//! let manager = GitManagerFacade::default();
//! let repo = PathBuf::from("/path/to/repo");
//!
//! // Stage every change (equivalent to `git add -A`).
//! let staged = manager.stage(&StageRequest::all(repo.clone()))?;
//! println!("staged {} pathspecs", staged.staged_pathspec_count);
//!
//! // Create a commit (author identity falls back to the repository's git config).
//! let commit = manager.commit(&CommitRequest {
//!     repository_path: repo.clone(),
//!     message: "feat: add a new module".to_string(),
//!     author_name: None,
//!     author_email: None,
//!     empty_commit_policy: EmptyCommitPolicy::Reject,
//!     hooks_policy: HooksPolicy { fail_if_hooks_present: false },
//! })?;
//! println!("commit: {}", commit.commit_oid);
//!
//! // Push to the remote.
//! let push = manager.push(&PushRequest {
//!     repository_path: repo,
//!     remote_name: "origin".to_string(),
//!     branch: Some("main".to_string()),
//!     refspecs: vec![],
//!     mirror: false,
//!     prune: false,
//!     force_with_lease: None,
//!     hooks_policy: HooksPolicy { fail_if_hooks_present: false },
//! })?;
//! println!("updated refs: {:?}", push.updated_refs);
//! # Ok::<(), avacana_gm::GitError>(())
//! ```
//!
//! ### Async operations
//!
//! Every synchronous method has an `*_async` counterpart. Async methods take the request
//! **by value** (the synchronous methods take it by reference):
//!
//! ```rust,no_run
//! use avacana_gm::{GitManagerFacade, CloneRequest, FetchTagMode};
//! use std::path::PathBuf;
//!
//! # async fn run() -> avacana_gm::GitResult<()> {
//! let manager = GitManagerFacade::default();
//! let request = CloneRequest {
//!     repository_url: "https://github.com/user/repo.git".to_string(),
//!     destination_path: PathBuf::from("/tmp/async-clone"),
//!     branch: None,
//!     depth: Some(1),
//!     tag_mode: FetchTagMode::Auto,
//!     mirror: false,
//! };
//!
//! let result = manager.clone_repository_async(request).await?;
//! println!("cloned into {:?}", result.repository_path);
//! # Ok(())
//! # }
//! ```
//!
//! ## Entry points
//!
//! | Item | Role |
//! |------|------|
//! | [`GitManager`] | The public trait that defines every domain operation. Depend on this in your own abstractions. |
//! | [`GitManagerFacade`] | The concrete `GitManager` implementation, plus the `*_async` methods. Cheap to [`Clone`] and `Send + Sync`. |
//! | [`GitManagerComponents`] | Typed dependency-injection container. Use [`GitManagerComponents::production`] for the full stack, or [`GitManagerComponents::new`] to assemble custom parts. |
//!
//! ## Operation surface
//!
//! All operations are declared on the [`GitManager`] trait:
//!
//! - **Remote** — [`clone_repository`](GitManager::clone_repository),
//!   [`fetch`](GitManager::fetch), [`pull`](GitManager::pull), [`push`](GitManager::push),
//!   [`ls_remote`](GitManager::ls_remote).
//! - **Local workflow** — [`stage`](GitManager::stage), [`commit`](GitManager::commit),
//!   [`create_branch`](GitManager::create_branch), [`switch_branch`](GitManager::switch_branch),
//!   [`merge`](GitManager::merge).
//! - **History rewrite** — [`history_rewrite`](GitManager::history_rewrite) (rebase,
//!   cherry-pick, revert, including continue/abort/skip).
//! - **Inspection** — [`status_diff`](GitManager::status_diff),
//!   [`line_diff`](GitManager::line_diff),
//!   [`working_copy_status`](GitManager::working_copy_status),
//!   [`working_copy_overview`](GitManager::working_copy_overview),
//!   [`repository_descriptor`](GitManager::repository_descriptor),
//!   [`scm_overview`](GitManager::scm_overview),
//!   [`tag_summaries`](GitManager::tag_summaries).
//! - **References and metadata** — [`refs`](GitManager::refs) (branches, tags, references,
//!   reflog, notes, transactions).
//! - **Plumbing** — [`plumbing`](GitManager::plumbing) (objects, trees, the index, and packs).
//! - **Advanced and queries** — [`advanced`](GitManager::advanced) (stash, submodules,
//!   worktrees, attributes) and [`query_lifecycle`](GitManager::query_lifecycle) (blame, log,
//!   show, revparse, init, and more).
//!
//! ## Asynchronous API
//!
//! The `*_async` methods are not natively asynchronous: they move the blocking libgit2 work
//! onto Tokio's blocking thread pool via [`tokio::task::spawn_blocking`], which keeps the async
//! runtime's reactor free. Each call clones the (cheap, `Arc`-backed) facade. A panic inside the
//! blocking task is surfaced as a [`GitErrorCode::Internal`] error.
//!
//! Read-only operations (`working_copy_status`, `working_copy_overview`,
//! `repository_descriptor`, `scm_overview`, `tag_summaries`) take no filesystem lock and may run
//! concurrently against the same repository; mutating operations are serialized per repository.
//!
//! ## Authentication
//!
//! Credentials are resolved by an ordered chain of providers. The chain stops at the first
//! provider that yields usable material, and an attempt budget bounds the total number of tries
//! so a misbehaving remote cannot loop forever. Host-key verification for SSH is performed
//! against `~/.ssh/known_hosts` (strict by default).
//!
//! The active provider set depends on the detected environment mode:
//!
//! | Mode | When | Interactive prompts |
//! |------|------|:-------------------:|
//! | `DesktopFull` | default on a developer machine | allowed (opt-in) |
//! | `HeadlessCi` | `CI=true` or `AVACANA_GM_AUTH_HEADLESS=1` | disabled |
//! | `RestrictedSandbox` | `AVACANA_GM_AUTH_RESTRICTED=1` | disabled |
//!
//! The mode can be forced with `AVACANA_GM_AUTH_ENV_MODE`. See
//! [Environment variables](#environment-variables) for the full list of knobs.
//!
//! ## Concurrency and locking
//!
//! Access is mediated by [`GitLockManager`], which hands out a [`GitLockGuard`] in one of two
//! modes ([`GitLockMode`]):
//!
//! - [`GitLockMode::ReadOnly`] — no filesystem lock; concurrent reads are allowed.
//! - [`GitLockMode::Mutating`] — an exclusive process-local lock **and** a filesystem lock file
//!   in the repository, so concurrent writers (within the process or across processes) are
//!   rejected with [`GitErrorCode::LockContention`].
//!
//! The guard releases both levels automatically on drop (RAII), or eagerly via
//! [`GitLockGuard::release`]. Sharing a single `Arc<GitManagerFacade>` across threads and tasks
//! is the intended usage pattern.
//!
//! ## Errors and retries
//!
//! Every operation returns [`GitResult<T>`], an alias for `Result<T, GitError>`. A
//! [`GitError`] carries a machine-readable [`GitErrorCode`] and may carry a
//! [`GitErrorRetryClassification`] (`Retryable` vs `Permanent`). The transport layer already
//! retries transient network failures with exponential backoff; the classification lets callers
//! layer their own retry policy on top. Some operations also return non-fatal
//! [`GitWarning`]s (for example, when hooks were detected but intentionally not executed).
//!
//! ```rust,no_run
//! use avacana_gm::{GitManager, GitManagerFacade, GitErrorCode, PushRequest, HooksPolicy};
//! use std::path::PathBuf;
//!
//! let manager = GitManagerFacade::default();
//! let request = PushRequest {
//!     repository_path: PathBuf::from("/path/to/repo"),
//!     remote_name: "origin".to_string(),
//!     branch: Some("main".to_string()),
//!     refspecs: vec![],
//!     mirror: false,
//!     prune: false,
//!     force_with_lease: None,
//!     hooks_policy: HooksPolicy { fail_if_hooks_present: false },
//! };
//!
//! match manager.push(&request) {
//!     Ok(result) => println!("updated refs: {:?}", result.updated_refs),
//!     Err(err) => match err.code() {
//!         GitErrorCode::AuthDenied => eprintln!("authentication failed: {err}"),
//!         GitErrorCode::PushRejectedRefs => eprintln!("remote rejected the push: {err}"),
//!         GitErrorCode::TransportTemporaryNetwork => eprintln!("transient network error: {err}"),
//!         other => eprintln!("[{other:?}] {err}"),
//!     },
//! }
//! ```
//!
//! ## Cargo features
//!
//! | Feature | Default | Description |
//! |---------|:-------:|-------------|
//! | `trace_logs` | off | Adds `tracing::instrument` spans to the public methods. Trace logging is compiled in only for debug builds; release builds strip it at compile time. |
//!
//! ## Environment variables
//!
//! All knobs use the `AVACANA_GM_` prefix.
//!
//! | Variable | Values | Purpose |
//! |----------|--------|---------|
//! | `AVACANA_GM_AUTH_ENV_MODE` | `desktop_full`, `headless_ci`, `restricted_sandbox` | Force the environment mode. |
//! | `AVACANA_GM_AUTH_RESTRICTED` | `1` | Select the `RestrictedSandbox` mode. |
//! | `AVACANA_GM_AUTH_HEADLESS` | `1` | Select the `HeadlessCi` mode (so does the standard `CI=true`). |
//! | `AVACANA_GM_AUTH_ALLOW_SSH` | `1` / `0` | Enable or disable the SSH transport. |
//! | `AVACANA_GM_AUTH_ALLOW_HTTPS` | `1` / `0` | Enable or disable the HTTPS transport. |
//! | `AVACANA_GM_AUTH_ALLOW_INTERACTIVE` | `1` / `0` | Allow interactive credential entry. |
//! | `AVACANA_GM_AUTH_ACCEPT_NEW_HOST` | `1` | Accept unknown SSH host keys (like `StrictHostKeyChecking=accept-new`). Strict by default. |
//! | `AVACANA_GM_GIT_AUTH_INTERACTIVE_OPT_IN` | `1` | Enable the interactive provider. |
//! | `AVACANA_GM_GIT_AUTH_INTERACTIVE_USERNAME` / `_PASSWORD` | string | Credentials for the interactive provider. |
//! | `AVACANA_GM_GIT_AUTH_OS_STORE_HOST` / `_USERNAME` / `_PASSWORD` / `_PORT` / `_PATH_PREFIX` | string | OS secret-store lookup parameters. |
//! | `AVACANA_GM_GIT_PROXY_URL` / `AVACANA_GM_GIT_PROXY_MODE` | string | Transport proxy configuration. |
//! | `AVACANA_GM_GIT_REMOTE_REDIRECT` | string | Remote redirect policy. |
//!
//! Standard Git and SSH variables (`SSH_AUTH_SOCK`, `GIT_CONFIG_GLOBAL`, `GIT_CREDENTIALS_PATH`,
//! `XDG_CONFIG_HOME`, `GIT_TERMINAL_PROMPT`, `DBUS_SESSION_BUS_ADDRESS`, …) are honored as well.
//!
//! [`git2`]: https://docs.rs/git2
//! [`libgit2`]: https://libgit2.org

#[path = "git_manager/mod.rs"]
mod git_manager;

#[doc(inline)]
pub use git_manager::*;
