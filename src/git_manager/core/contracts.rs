#[path = "contracts_errors.rs"]
mod errors;
#[path = "contracts_local_workflow.rs"]
mod local_workflow;
#[path = "contracts_remote.rs"]
mod remote;

pub use errors::{
    GitError, GitErrorCode, GitErrorRetryClassification, GitResult, GitWarning, GitWarningCode,
};
pub use local_workflow::{
    CommitRequest, CommitResult, CreateBranchRequest, CreateBranchResult, EmptyCommitPolicy,
    HooksPolicy, MergeRequest, MergeResult, StageRequest, StageResult, SwitchBranchRequest,
    SwitchBranchResult,
};
pub use remote::{
    CloneRequest, CloneResult, FetchRequest, FetchResult, FetchTagMode, ForceWithLeasePolicy,
    ForceWithLeaseRef, LsRemoteRequest, LsRemoteResult, MergeFileFavor, PullMode, PullRequest,
    PullResult, PushRequest, PushResult, RemoteReference,
};
