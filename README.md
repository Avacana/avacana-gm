# avacana-gm

A standalone Rust crate for embedding Git operations into your application. It replaces
shelling out to the `git` command-line tool with native, in-process operations through
`libgit2`, while preserving command-line Git semantics.

## Key features

- **Full Git operation surface** — clone, fetch, pull, push, stage, commit, branch, switch,
  merge, rebase, cherry-pick, revert, stash, diff, status, refs, plumbing, and more.
- **Native implementation** — every operation runs through `git2` (libgit2); the `git` binary
  is never spawned.
- **Layered authentication** — an ordered chain of credential providers: SSH agent, SSH config,
  SSH identity files, URL credentials, git config, OS keychain, git-credentials files, netrc,
  and an opt-in interactive provider.
- **Thread safety** — a two-level locking system (process-local plus filesystem); the entire
  public API is `Send + Sync`.
- **Async support** — every operation has an `*_async` variant backed by
  `tokio::task::spawn_blocking`.
- **Retry strategies** — transient network failures are retried automatically with exponential
  backoff.
- **Structured logging** — optional tracing via the `trace_logs` feature.
- **Typed API** — a dedicated request/result pair for every operation.

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
avacana-gm = "0.1"
```

### Cargo features

| Feature | Description | Default |
|---------|-------------|:-------:|
| `trace_logs` | Adds `tracing::instrument` spans to the public methods (debug builds only). | off |

```toml
[dependencies]
avacana-gm = { version = "0.1", features = ["trace_logs"] }
```

## Quick start

### Creating a `GitManager`

```rust
use avacana_gm::{GitManagerFacade, GitManager};

// Option 1: production build with the full authentication chain.
let manager = GitManagerFacade::default();

// Option 2: through the explicit dependency-injection container (recommended).
use avacana_gm::GitManagerComponents;
let manager = GitManagerFacade::new(GitManagerComponents::production());
```

### Cloning a repository

```rust
use avacana_gm::{GitManagerFacade, GitManager, CloneRequest, FetchTagMode};
use std::path::PathBuf;

let manager = GitManagerFacade::default();

let request = CloneRequest {
    repository_url: "https://github.com/user/repo.git".to_string(),
    destination_path: PathBuf::from("/tmp/my-clone"),
    branch: Some("main".to_string()),
    depth: None,            // full clone
    tag_mode: FetchTagMode::Auto,
    mirror: false,
};

let result = manager.clone_repository(&request)?;
println!("cloned into: {:?}", result.repository_path);
println!("branch: {:?}", result.checked_out_branch);
```

### Working-tree status

```rust
use avacana_gm::{
    GitManagerFacade, GitManager, WorkingCopyStatusRequest,
    WorkingCopyScope, WorkingCopyEntryKind,
};
use std::path::PathBuf;

let manager = GitManagerFacade::default();

let request = WorkingCopyStatusRequest::new(
    PathBuf::from("/path/to/repo"),
    WorkingCopyScope::Full,
    true,   // include_untracked
    false,  // include_ignored
    true,   // detect_renames
    false,  // detect_copies
    false,  // include_directories
);

let result = manager.working_copy_status(&request)?;
println!("repository: {:?}", result.repository.repo_root);
for entry in &result.entries {
    match &entry.kind {
        WorkingCopyEntryKind::Tracked { index, worktree, .. } => {
            println!("  {} — index: {:?}, worktree: {:?}", entry.path, index, worktree);
        }
        WorkingCopyEntryKind::Untracked => println!("  {} — untracked", entry.path),
        WorkingCopyEntryKind::Ignored => println!("  {} — ignored", entry.path),
    }
}
```

### Stage + commit + push

```rust
use avacana_gm::{
    GitManagerFacade, GitManager,
    StageRequest, CommitRequest, PushRequest,
    EmptyCommitPolicy, HooksPolicy,
};
use std::path::PathBuf;

let manager = GitManagerFacade::default();
let repo = PathBuf::from("/path/to/repo");

// Stage every change.
let stage_result = manager.stage(&StageRequest::all(repo.clone()))?;
println!("staged {} pathspecs", stage_result.staged_pathspec_count);

// Commit.
let commit_result = manager.commit(&CommitRequest {
    repository_path: repo.clone(),
    message: "feat: add a new module".to_string(),
    author_name: None,     // from git config
    author_email: None,    // from git config
    empty_commit_policy: EmptyCommitPolicy::Reject,
    hooks_policy: HooksPolicy { fail_if_hooks_present: false },
})?;
println!("commit: {}", commit_result.commit_oid);

// Push.
let push_result = manager.push(&PushRequest {
    repository_path: repo,
    remote_name: "origin".to_string(),
    branch: Some("main".to_string()),
    refspecs: vec![],
    mirror: false,
    prune: false,
    force_with_lease: None,
    hooks_policy: HooksPolicy { fail_if_hooks_present: false },
})?;
println!("updated refs: {:?}", push_result.updated_refs);
```

### Async operations

Every operation has an `*_async` variant. Async methods take the request **by value** (the
synchronous methods take it by reference):

```rust
use avacana_gm::{GitManagerFacade, CloneRequest, FetchTagMode};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = GitManagerFacade::default();

    let request = CloneRequest {
        repository_url: "https://github.com/user/repo.git".to_string(),
        destination_path: PathBuf::from("/tmp/async-clone"),
        branch: None,
        depth: Some(1),  // shallow clone
        tag_mode: FetchTagMode::Auto,
        mirror: false,
    };

    let result = manager.clone_repository_async(request).await?;
    println!("async clone: {:?}", result.repository_path);
    Ok(())
}
```

## Error handling

Operations return `GitResult<T>` (an alias for `Result<T, GitError>`). A `GitError` exposes a
machine-readable `GitErrorCode` and may carry a `GitErrorRetryClassification` (`Retryable` vs
`Permanent`):

```rust
use avacana_gm::{GitManager, GitManagerFacade, GitErrorCode};
# use avacana_gm::PushRequest;
# fn demo(manager: &GitManagerFacade, request: &PushRequest) {
match manager.push(request) {
    Ok(result) => println!("updated refs: {:?}", result.updated_refs),
    Err(err) => match err.code() {
        GitErrorCode::AuthDenied => eprintln!("authentication failed: {err}"),
        GitErrorCode::PushRejectedRefs => eprintln!("remote rejected the push: {err}"),
        GitErrorCode::TransportTemporaryNetwork => eprintln!("transient network error: {err}"),
        other => eprintln!("[{other:?}] {err}"),
    },
}
# }
```

## Project layout

```
avacana-gm/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # crate entry point; re-exports git_manager::*
│   └── git_manager/
│       ├── mod.rs                # root module, public re-exports
│       ├── facade.rs             # the GitManager trait + GitManagerFacade
│       ├── facade_sync.rs        # synchronous impl (delegates to the pipeline)
│       ├── facade_async.rs       # async wrappers via tokio::spawn_blocking
│       ├── composition.rs        # DI container: GitManagerComponents
│       ├── auth/                 # authentication subsystem (provider chain)
│       ├── transport/            # git2 transport bridge (callbacks, retries, host keys)
│       ├── state/                # locking and concurrency policy
│       ├── core/                 # pipeline + the operation implementations
│       └── diagnostics/          # diagnostics
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `git2` | 0.20 | libgit2 bindings (vendored libgit2 + OpenSSL) |
| `tokio` | 1 | async runtime for `spawn_blocking` |
| `tracing` | 0.1 | structured logging (optional, via `trace_logs`) |
| `uuid` | 1 | UUID v4 generation for lock tokens |
| `base64` | 0.22 | encoding for SSH host-key verification |
| `dirs` | 5 | home-directory resolution |
| `glob` | 0.3 | pattern matching for pathspecs |

## Documentation

The full API reference is published on [docs.rs](https://docs.rs/avacana-gm). The crate-level
documentation (run `cargo doc --open`) covers the entry points, the operation surface,
authentication, concurrency and locking, error handling, the Cargo features, and the
`AVACANA_GM_*` environment variables.

## License

`avacana-gm` is distributed under the [MIT](LICENSE) license.

It links against native libraries under their own terms — most notably **libgit2**
(GPL-2.0 *with a linking exception*) and **OpenSSL** (Apache-2.0). The linking
exception lets MIT-licensed and proprietary software link libgit2 freely; if you
redistribute compiled artifacts, see **[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)**
for the attribution and source-availability obligations that apply to those bundled
libraries.

Copyright (c) 2026 Avacana Dhatu Team
