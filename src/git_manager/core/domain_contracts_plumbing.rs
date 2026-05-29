//! Contracts of the `plumbing` domain.

use std::path::PathBuf;

/// Request for the plumbing domain (`odb/object/treebuilder/indexer/packbuilder`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlumbingRequest {
    /// Path to the local repository.
    pub repository_path: PathBuf,
    /// The plumbing operation to perform.
    pub operation: PlumbingOperation,
}

/// Typed plumbing-level operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlumbingOperation {
    /// Read an object by OID.
    ReadObject { oid: String },
    /// Stream-write a blob object from chunks (`BlobWriter`/`OdbWriter`).
    WriteBlobStreamFromChunks {
        /// Optional hint path for `BlobWriter` filters.
        hint_path: Option<PathBuf>,
        /// Stream of chunks to write the blob from.
        chunks: Vec<Vec<u8>>,
        /// `true` = use `OdbWriter`, `false` = use `BlobWriter`.
        use_odb_writer: bool,
    },
    /// Stream-read a blob object (`OdbReader`).
    ReadBlobStream {
        /// OID of the blob object.
        oid: String,
        /// Read-chunk size for streaming reads.
        chunk_size: usize,
    },
    /// Write a blob object from a file (`hash-object -w`).
    WriteBlobFromPath { source_path: PathBuf },
    /// Compute a blob OID without writing to the ODB (`hash-object` without `-w`).
    HashBlobFromPath { source_path: PathBuf },
    /// Build a tree from a set of entries.
    BuildTree {
        /// Base tree, if an incremental update is needed.
        base_tree: Option<String>,
        /// List of tree entries.
        entries: Vec<TreeEntrySpec>,
    },
    /// Synchronize the index with an optional tree and write the resulting tree object.
    IndexSnapshot {
        /// Tree source to preload into the index (`read-tree`).
        source_tree: Option<String>,
    },
    /// Inspect the typed representation of index entries/conflicts/pathspec matches.
    InspectIndexEntriesAndConflicts {
        /// Optional prefix for `Index::find_prefix`.
        prefix: Option<String>,
        /// Set of pathspecs for dry-run inspection via `IndexMatchedPath`.
        pathspecs: Vec<String>,
        /// Optional commit OID/revision pair for merge-index conflict inspection.
        /// First element: `our`, second element: `their`.
        conflict_pair: Option<(String, String)>,
    },
    /// Prepare a pack stream for the given refs.
    BuildPack {
        /// Set of refs to include in the pack.
        include_references: Vec<String>,
    },
    /// Prepare a pack stream with explicit settings (`pack-objects` option bridge).
    BuildPackWithOptions {
        /// Set of refs to include in the pack.
        include_references: Vec<String>,
        /// Explicit packbuilder thread count (`None` = determine automatically).
        threads: Option<usize>,
    },
    /// Index a pack file (`index-pack`).
    IndexPack { pack_path: PathBuf },
    /// Index a pack file with explicit settings.
    IndexPackWithOptions {
        /// Path to the pack file.
        pack_path: PathBuf,
        /// Allow thin-pack fixup via the local ODB.
        fix_thin: bool,
    },
    /// Index a pack stream already loaded into memory (network scenario).
    IndexPackStream { pack_data: Vec<u8> },
    /// Index a pack stream with explicit settings.
    IndexPackStreamWithOptions {
        /// Binary data of the pack stream.
        pack_data: Vec<u8>,
        /// Allow thin-pack fixup via the local ODB.
        fix_thin: bool,
    },
}

/// Entry specification for treebuilder operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntrySpec {
    /// Repository path of the entry.
    pub path: String,
    /// Object OID.
    pub object_id: String,
    /// File mode in git format.
    pub file_mode: u32,
}

/// Stages of building a pack file via `PackBuilder`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackBuildStage {
    /// Stage of adding objects to the pack.
    AddingObjects,
    /// Stage of pack delta compression.
    Deltafication,
}

/// Progress snapshot of a packbuilder operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackBuildProgress {
    /// Last observed stage of the packbuilder process.
    pub stage: Option<PackBuildStage>,
    /// Current progress value of the stage.
    pub current: u32,
    /// Total progress value of the stage.
    pub total: u32,
    /// Number of objects written by the packbuilder.
    pub written_objects: usize,
    /// Typed name of the operation that produced this payload.
    pub operation: Option<String>,
    /// Backend of the streaming operation (`blob_writer` / `odb_writer` / `odb_reader` / `odb_packwriter`).
    pub stream_backend: Option<String>,
    /// Total number of processed stream chunks.
    pub stream_chunk_count: Option<usize>,
    /// Read-chunk size for streaming reads.
    pub stream_chunk_size: Option<usize>,
    /// Number of processed stream bytes.
    pub stream_bytes: Option<usize>,
    /// Typed snapshot of index entries (`IndexEntries`/`IndexEntry`).
    pub index_entries: Vec<TreeEntrySpec>,
    /// Typed snapshot of conflicted index entries (`IndexConflicts`/`IndexConflict`).
    pub index_conflicts: Vec<(
        Option<TreeEntrySpec>,
        Option<TreeEntrySpec>,
        Option<TreeEntrySpec>,
    )>,
    /// Typed snapshot of `IndexMatchedPath` callback matches (`path`, `pathspec`).
    pub index_matched_paths: Vec<(String, String)>,
    /// Position of the first prefix match in the index (`Index::find_prefix`).
    pub index_prefix_position: Option<usize>,
    /// Size of the mempack dump stream in bytes.
    pub mempack_dump_size: Option<usize>,
    /// `true` if the object was restored via `OdbPackwriter` after a mempack reset.
    pub mempack_restored: Option<bool>,
}

/// Progress snapshot of indexing a pack file via `Indexer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IndexerProgressSnapshot {
    /// Number of objects in the pack.
    pub total_objects: usize,
    /// Number of objects already indexed.
    pub indexed_objects: usize,
    /// Number of objects received.
    pub received_objects: usize,
    /// Number of locally found objects for the thin-pack.
    pub local_objects: usize,
    /// Total number of deltas in the pack.
    pub total_deltas: usize,
    /// Number of deltas already indexed.
    pub indexed_deltas: usize,
    /// Number of received bytes of the pack stream.
    pub received_bytes: usize,
}

/// Result of the plumbing domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlumbingResult {
    /// OID of the resulting object/artifact.
    pub object_id: Option<String>,
    /// Git object type (`blob/tree/commit/tag`).
    pub object_kind: Option<String>,
    /// Object size in bytes.
    pub object_size: Option<usize>,
    /// Number of index entries after the index operation.
    pub index_entry_count: Option<usize>,
    /// Number of indexed objects.
    pub indexed_objects: usize,
    /// Number of objects placed into the pack.
    pub packed_objects: usize,
    /// Typed progress/inspection payload for pack/stream/index operations.
    pub pack_progress: Option<PackBuildProgress>,
    /// Progress snapshot of the indexer/packwriter (if a streaming pack operation ran).
    pub indexer_progress: Option<IndexerProgressSnapshot>,
}
