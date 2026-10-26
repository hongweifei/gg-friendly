//! Request/response types for read-only operations like log listing and
//! revision details.

use super::*;

/// A file-level change within a revision's diff.
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub struct RevChange {
    pub kind: ChangeKind,
    pub path: TreePath,
    pub has_conflict: bool,
    pub hunks: Vec<ChangeHunk>,
}

/// A file-level conflict in a revision's parent tree.
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub struct RevConflict {
    pub path: TreePath,
    pub hunk: ChangeHunk,
}

/// The type of modification made to a file in a diff.
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub enum ChangeKind {
    None,
    Added,
    Deleted,
    Modified,
}

/// Response to a revision detail query: either not found or full details.
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
#[allow(clippy::large_enum_variant)]
pub enum RevsResult {
    NotFound {
        set: RevSet,
    },
    Detail {
        /// The revision set that was queried.
        set: RevSet,
        /// All revisions in the set, ordered ancestors-first.
        headers: Vec<RevHeader>,
        /// Parents of the oldest revision in the set.
        parents: Vec<RevHeader>,
        /// Combined changes: diff from oldest parent to newest revision.
        changes: Vec<RevChange>,
        /// Conflicts present in the parent tree of the oldest revision.
        conflicts: Vec<RevConflict>,
    },
}

/// (column, row) position of a node in the log graph.
#[derive(Serialize, Clone, Copy, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub struct LogCoordinates(pub usize, pub usize);

/// An edge segment in the log graph connecting two nodes.
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub enum LogLine {
    FromNode {
        source: LogCoordinates,
        target: LogCoordinates,
        indirect: bool,
    },
    ToNode {
        source: LogCoordinates,
        target: LogCoordinates,
        indirect: bool,
    },
    ToIntersection {
        source: LogCoordinates,
        target: LogCoordinates,
        indirect: bool,
    },
    ToMissing {
        source: LogCoordinates,
        target: LogCoordinates,
        indirect: bool,
    },
}

/// A single row in the log graph with its revision and edges.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub struct LogRow {
    pub revision: RevHeader,
    pub location: LogCoordinates,
    pub padding: usize,
    pub lines: Vec<LogLine>,
}

/// A paginated slice of the log graph.
#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub struct LogPage {
    pub rows: Vec<LogRow>,
    pub has_more: bool,
}

/// A branch in the repository, either local or on a remote.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS), ts(export, export_to = "app/messages/"))]
pub struct BranchInfo {
    /// The branch name. For a remote branch this is the name without the
    /// remote, so `main` and `main@origin` can be told apart by `is_local`.
    pub name: String,
    /// Whether this is a local branch.
    pub is_local: bool,
    /// The remote this branch lives on, if it is a remote branch.
    pub remote_name: Option<String>,
    /// The commit the branch points at.
    pub target: Option<CommitId>,
    /// For a local branch: whether it tracks a remote branch. For a remote
    /// branch: whether the local branch of the same name tracks it.
    pub is_tracking: bool,
    /// Whether the branch matches the remote it tracks.
    pub is_synced: bool,
    /// Whether the branch was deleted on the remote.
    pub is_absent: bool,
    /// Whether the branch points at more than one commit.
    pub has_conflict: bool,
    /// How many commits the local branch has that the tracked remote doesn't.
    /// Only for local branches that track a remote.
    pub ahead: Option<usize>,
    /// How many commits the tracked remote has that the local branch doesn't.
    /// Only for local branches that track a remote.
    pub behind: Option<usize>,
}
