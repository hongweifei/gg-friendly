use std::{borrow::Borrow, collections::HashSet, io::Write, mem, ops::Range, pin::Pin};

use anyhow::{Result, anyhow};
use futures_util::{AsyncReadExt, Stream, StreamExt, try_join};
use gix::bstr::ByteVec;
use itertools::Itertools;
use jj_cli::diff_util::LineDiffOptions;
use jj_lib::{
    backend::{CommitId, MergedTreeValueExt as _},
    conflict_labels::ConflictLabels,
    conflicts::{
        self, ConflictMarkerStyle, ConflictMaterializeOptions, MaterializedFileValue,
        MaterializedTreeValue,
    },
    diff::{
        CompareBytesExactly, CompareBytesIgnoreAllWhitespace, CompareBytesIgnoreWhitespaceAmount,
        ContentDiff, DiffHunk, DiffHunkKind, find_line_ranges,
    },
    diff_presentation::LineCompareMode,
    files::FileMergeHunkLevel,
    git::REMOTE_NAME_FOR_LOCAL_GIT_REPO,
    graph::{GraphEdgeType, GraphNode, TopoGroupedGraph},
    matchers::EverythingMatcher,
    merge::{Diff, SameChange},
    merged_tree::{TreeDiffEntry, TreeDiffStream},
    op_store::RefTarget,
    ref_name::{RefNameBuf, RemoteNameBuf, RemoteRefSymbol},
    repo::Repo,
    repo_path::RepoPath,
    revset::{self, Revset, RevsetContainingFn, RevsetEvaluationError, RevsetStreamExt},
    rewrite,
    tree_merge::MergeOptions,
};
use pollster::FutureExt as _;

use crate::messages::{
    ChangeHunk, ChangeLocation, ChangeRange, MultilineString, RevSet, queries::*,
};
#[cfg(test)]
use crate::messages::{RevHeader, RevId};

use super::{WorkspaceSession, git_util::get_git_remote_names};

struct LogStem {
    source: LogCoordinates,
    target: CommitId,
    indirect: bool,
    was_inserted: bool,
    known_immutable: bool,
}

/// state used for init or restart of a query
pub struct QueryState {
    /// max number of rows per page
    page_size: usize,
    /// number of rows already yielded
    next_row: usize,
    /// ongoing vertical lines; nodes will be placed on or around these
    stems: Vec<Option<LogStem>>,
    /// stem of a missing edge, kept until the next row so that nothing is drawn under its stub
    missing_stem: Option<usize>,
}

impl QueryState {
    pub fn new(page_size: usize) -> QueryState {
        QueryState {
            page_size,
            next_row: 0,
            stems: Vec::new(),
            missing_stem: None,
        }
    }
}

/// live instance of a query
pub struct QuerySession<'q, 'w: 'q> {
    pub ws: &'q WorkspaceSession<'w>,
    pub state: QueryState,
    #[allow(clippy::type_complexity)]
    stream: Pin<Box<dyn Stream<Item = Result<GraphNode<CommitId>, RevsetEvaluationError>> + 'q>>,
    /// pre-fetched node used for has_more peek semantics
    lookahead: Option<Result<GraphNode<CommitId>, RevsetEvaluationError>>,
    /// once the underlying stream returns None we must not poll it again
    stream_done: bool,
    is_immutable: Box<RevsetContainingFn<'q>>,
}

impl<'q, 'w> QuerySession<'q, 'w> {
    pub fn new(
        ws: &'q WorkspaceSession<'w>,
        revset: &'q dyn Revset,
        state: QueryState,
        prioritized: &[CommitId],
    ) -> QuerySession<'q, 'w> {
        let as_id: for<'a> fn(&'a CommitId) -> &'a CommitId = commit_id_identity;
        let mut graph = TopoGroupedGraph::new(revset.stream_graph(), as_id);
        // the branches people work on come first, so the history they care
        // about is not pushed off the screen by a chain of conflicts
        for id in prioritized {
            graph.prioritize_branch(id.clone());
        }
        let mut stream: Pin<Box<dyn Stream<Item = _> + 'q>> = Box::pin(graph.stream());
        // skip already-yielded rows
        let mut stream_done = false;
        for _ in 0..state.next_row {
            match stream.next().block_on() {
                Some(_) => {}
                None => {
                    stream_done = true;
                    break;
                }
            }
        }

        let immutable_revset = ws.evaluate_immutable().unwrap();
        let is_immutable = immutable_revset.containing_fn();

        QuerySession {
            ws,
            stream,
            lookahead: None,
            stream_done,
            state,
            is_immutable,
        }
    }

    fn next_node(&mut self) -> Option<Result<GraphNode<CommitId>, RevsetEvaluationError>> {
        if let Some(item) = self.lookahead.take() {
            return Some(item);
        }
        if self.stream_done {
            return None;
        }
        match self.stream.next().block_on() {
            Some(item) => Some(item),
            None => {
                self.stream_done = true;
                None
            }
        }
    }

    fn has_more(&mut self) -> bool {
        if self.lookahead.is_some() {
            return true;
        }
        if self.stream_done {
            return false;
        }
        match self.stream.next().block_on() {
            Some(item) => {
                self.lookahead = Some(item);
                true
            }
            None => {
                self.stream_done = true;
                false
            }
        }
    }

    pub fn get_page(&mut self) -> Result<LogPage> {
        let mut rows: Vec<LogRow> = Vec::with_capacity(self.state.page_size); // output rows to draw
        let mut row = self.state.next_row;
        let max = row + self.state.page_size;

        let root_id = self.ws.repo().store().root_commit_id().clone();

        while let Some(Ok((commit_id, commit_edges))) = self.next_node() {
            // output lines to draw for the current row
            let mut lines: Vec<LogLine> = Vec::new();

            // find an existing stem targeting the current node
            let mut column = self.state.stems.len();
            let mut stem_known_immutable = false;
            let mut padding = 0; // used to offset the commit summary past some edges

            if let Some(slot) = self.find_stem_for_commit(&commit_id) {
                column = slot;
                padding = self.state.stems.len() - column - 1;
            }

            // terminate any existing stem, removing it from the end or leaving a gap
            if column < self.state.stems.len() {
                if let Some(terminated_stem) = &self.state.stems[column] {
                    stem_known_immutable = terminated_stem.known_immutable;
                    lines.push(if terminated_stem.was_inserted {
                        LogLine::FromNode {
                            indirect: terminated_stem.indirect,
                            source: terminated_stem.source,
                            target: LogCoordinates(column, row),
                        }
                    } else {
                        LogLine::ToNode {
                            indirect: terminated_stem.indirect,
                            source: terminated_stem.source,
                            target: LogCoordinates(column, row),
                        }
                    });
                }
                self.state.stems[column] = None;
            }
            // otherwise, slot into any gaps that might exist
            else {
                for (slot, stem) in self.state.stems.iter().enumerate() {
                    if stem.is_none() {
                        column = slot;
                        padding = self.state.stems.len() - slot - 1;
                        break;
                    }
                }
            }

            let known_immutable = if stem_known_immutable {
                Some(true)
            } else {
                Some((self.is_immutable)(&commit_id).block_on()?)
            };

            let header = self
                .ws
                .format_header(&self.ws.get_commit(&commit_id)?, known_immutable)?;

            // remove empty stems on the right edge
            let empty_stems = self
                .state
                .stems
                .iter()
                .rev()
                .take_while(|stem| stem.is_none())
                .count();
            self.state
                .stems
                .truncate(self.state.stems.len() - empty_stems);

            // merge edges into existing stems or add new ones to the right
            let mut next_missing: Option<CommitId> = None;
            'edges: for edge in commit_edges.iter() {
                if edge.edge_type == GraphEdgeType::Missing {
                    if edge.target == root_id {
                        continue;
                    } else {
                        next_missing = Some(edge.target.clone());
                    }
                }

                let indirect = edge.edge_type != GraphEdgeType::Direct;

                for (slot, stem) in self.state.stems.iter().enumerate() {
                    if let Some(stem) = stem
                        && stem.target == edge.target
                        && self.state.missing_stem != Some(slot)
                    {
                        lines.push(LogLine::ToIntersection {
                            indirect,
                            source: LogCoordinates(column, row),
                            target: LogCoordinates(slot, row + 1),
                        });
                        continue 'edges;
                    }
                }

                for stem in self.state.stems.iter_mut() {
                    if stem.is_none() {
                        *stem = Some(LogStem {
                            source: LogCoordinates(column, row),
                            target: edge.target.clone(),
                            indirect,
                            was_inserted: true,
                            known_immutable: header.is_immutable,
                        });
                        continue 'edges;
                    }
                }

                self.state.stems.push(Some(LogStem {
                    source: LogCoordinates(column, row),
                    target: edge.target.clone(),
                    indirect,
                    was_inserted: false,
                    known_immutable: header.is_immutable,
                }));
            }

            rows.push(LogRow {
                revision: header,
                location: LogCoordinates(column, row),
                padding,
                lines,
            });
            row += 1;

            if let Some(slot) = self.state.missing_stem.take() {
                self.state.stems[slot] = None;
            }

            // stub out missing edges into the next row, reserving the slot so nothing lands under the stub.
            // the stub can't have a row of its own, because the frontend treats locations as row indices
            if let Some(slot) = next_missing
                .take()
                .and_then(|id| self.find_stem_for_commit(&id))
            {
                if let Some(terminated_stem) = &self.state.stems[slot] {
                    rows.last_mut().unwrap().lines.push(LogLine::ToMissing {
                        indirect: terminated_stem.indirect,
                        source: LogCoordinates(column, row - 1),
                        target: LogCoordinates(slot, row),
                    });
                }
                self.state.missing_stem = Some(slot);
            };

            if row == max {
                break;
            }
        }

        self.state.next_row = row;
        let has_more = self.has_more();
        Ok(LogPage { rows, has_more })
    }

    fn find_stem_for_commit(&self, id: &CommitId) -> Option<usize> {
        for (slot, stem) in self.state.stems.iter().enumerate() {
            if let Some(LogStem { target, .. }) = stem
                && target == id
            {
                return Some(slot);
            }
        }

        None
    }
}

#[cfg(test)]
pub fn query_log(ws: &WorkspaceSession, revset_str: &str, max_results: usize) -> Result<LogPage> {
    let state = QueryState::new(max_results);
    let revset = ws.evaluate_revset_str(revset_str)?;
    let prioritized = prioritized_commits(ws, &*revset)?;
    let mut session = QuerySession::new(ws, &*revset, state, &prioritized);
    session.get_page()
}

/// The changes to show first: the working copy.
///
/// jj's own default walks into every conflict chain, so a repository that
/// once had a conflict shows that chain first. Putting the working copy
/// first is what jj itself does (`revsets.log-graph-prioritize`), and it
/// keeps the branch people are working on at the top of the list.
///
/// Only changes the query actually returns are picked, because the graph
/// needs every prioritised change to be part of the result.
pub fn prioritized_commits(ws: &WorkspaceSession, revset: &dyn Revset) -> Result<Vec<CommitId>> {
    let mut candidates = Vec::new();

    if let Some(wc) = ws.repo().view().get_wc_commit_id(ws.name()) {
        candidates.push(wc.clone());
    }

    // keep only the ones the query returns
    let store = ws.repo().store();
    let mut stream = revset.stream().commits(store);
    let mut in_result = std::collections::HashSet::new();
    while let Some(commit) = stream.next().block_on() {
        in_result.insert(commit?.id().clone());
    }

    Ok(candidates
        .into_iter()
        .filter(|id| in_result.contains(id))
        .collect())
}

#[cfg(test)]
pub fn query_revision(ws: &WorkspaceSession<'_>, id: &RevId) -> Result<Option<RevHeader>> {
    ws.resolve_optional_id(id)?
        .map(|c| ws.format_header(&c, None))
        .transpose()
}

/// Read display details for a revset (limited to sequences). Returns headers in topological order (children first).
pub async fn query_revisions(ws: &WorkspaceSession<'_>, set: RevSet) -> Result<RevsResult> {
    // resolve singleton or arbitrary revset
    let commits = if set.from.change.hex == set.to.change.hex {
        match ws.resolve_optional_id(&set.from)? {
            Some(commit) => vec![commit],
            None => return Ok(RevsResult::NotFound { set }),
        }
    } else {
        match ws.resolve_optional_set(&set)? {
            Some(commits) => commits,
            None => return Ok(RevsResult::NotFound { set }),
        }
    };

    // trees before and after the revset
    let oldest_commit = commits.last().ok_or(anyhow!("slice is_empty()"))?;
    let oldest_parents = oldest_commit.parents().await?;
    let parent_tree = rewrite::merge_commit_trees(ws.repo(), &oldest_parents).await?;

    let newest_commit = commits.first().ok_or(anyhow!("slice is_empty()"))?;
    let final_tree = newest_commit.tree();

    // compute combined changes: diff from parents to final
    let mut changes = Vec::new();
    let tree_diff = parent_tree.diff_stream(&final_tree, &EverythingMatcher);
    let conflict_labels = Diff::new(parent_tree.labels(), final_tree.labels());
    format_tree_changes(ws, &mut changes, tree_diff, conflict_labels).await?;

    let mut conflicts = Vec::new();
    for (path, entry) in final_tree.entries() {
        if let Ok(entry) = entry
            && !entry.is_resolved()
        {
            let conflict_labels = readable_labels(final_tree.labels());
            match conflicts::materialize_tree_value(
                ws.repo().store(),
                &path,
                entry,
                &conflict_labels,
            )
            .await?
            {
                MaterializedTreeValue::FileConflict(file) => {
                    let mut hunk_content = vec![];
                    conflicts::materialize_merge_result(
                        &file.contents,
                        &file.labels,
                        &mut hunk_content,
                        &ConflictMaterializeOptions {
                            marker_style: ConflictMarkerStyle::Git,
                            marker_len: None,
                            merge: MergeOptions {
                                hunk_level: FileMergeHunkLevel::Line,
                                same_change: SameChange::Accept,
                            },
                        },
                    )?;
                    let mut hunks = get_unified_hunks(3, &hunk_content, &[])?;
                    if let Some(hunk) = hunks.pop() {
                        conflicts.push(RevConflict {
                            path: ws.format_path(path)?,
                            hunk,
                        });
                    }
                }
                _ => {
                    log::warn!("nonresolved tree entry did not materialise as conflict");
                }
            }
        }
    }

    let conflicted_paths: HashSet<String> = conflicts
        .iter()
        .map(|conflict| conflict.path.repo_path.clone())
        .collect();
    changes.retain(|change| !conflicted_paths.contains(&change.path.repo_path));

    // details for each revision in the set
    let mut headers = Vec::new();
    let mut known_immutable: Option<bool> = None;
    for commit in &commits {
        // optimization: once we find an immutable revision, its ancestors must be immutable too
        let header = ws.format_header(commit, known_immutable)?;
        if known_immutable.is_none() && header.is_immutable {
            known_immutable = Some(true);
        }
        headers.push(header);
    }

    // optimization: if anything was immutable, the oldest revision's parents must also be immutable
    let parents = oldest_commit
        .parents()
        .await?
        .iter()
        .map(|p| ws.format_header(p, known_immutable))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(RevsResult::Detail {
        set,
        headers,
        parents,
        changes,
        conflicts,
    })
}

pub fn query_remotes(
    ws: &WorkspaceSession,
    tracking_bookmark: Option<String>,
) -> Result<Vec<String>> {
    let git_repo = match ws.git_repo() {
        Some(git_repo) => git_repo,
        None => return Err(anyhow!("No git backend")),
    };

    let all_remotes = get_git_remote_names(&git_repo);

    let matching_remotes = match tracking_bookmark {
        Some(bookmark_name) => all_remotes
            .into_iter()
            .filter(|remote_name| {
                let remote_name_ref = RemoteNameBuf::from(remote_name);
                let bookmark_name_ref = RefNameBuf::from(bookmark_name.clone());
                let remote_ref_symbol = RemoteRefSymbol {
                    name: &bookmark_name_ref,
                    remote: &remote_name_ref,
                };
                let remote_ref = ws.view().get_remote_bookmark(remote_ref_symbol);
                !remote_ref.is_absent() && remote_ref.is_tracked()
            })
            .collect(),
        None => all_remotes,
    };

    Ok(matching_remotes)
}

/// List every branch in the repository, local and remote.
///
/// The local branch and the remote branches it tracks are the same branch, so
/// they are reported separately and the frontend can group them.
pub fn query_branches(ws: &WorkspaceSession) -> Result<Vec<BranchInfo>> {
    let mut result = Vec::new();

    for (bookmark_name, bookmark_target) in ws.view().bookmarks() {
        let local_target = bookmark_target.local_target;
        let remote_refs = bookmark_target.remote_refs;

        if local_target.is_present() {
            // the local branch is synced when it matches every remote it tracks
            let is_synced = remote_refs.iter().all(|&(remote_name, remote_ref)| {
                remote_name == REMOTE_NAME_FOR_LOCAL_GIT_REPO
                    || !remote_ref.is_tracked()
                    || remote_ref.target == *local_target
            });

            // how far the local branch and the remote it tracks have drifted
            // apart, counted against the remote it actually tracks
            let tracked_remote = remote_refs.iter().find(|&&(remote_name, remote_ref)| {
                remote_name != REMOTE_NAME_FOR_LOCAL_GIT_REPO && remote_ref.is_tracked()
            });
            let (ahead, behind) = match tracked_remote {
                Some(&(_, remote_ref)) => count_ahead_behind(
                    ws,
                    local_target.present_adds().cloned().collect_vec(),
                    remote_ref.target.present_adds().cloned().collect_vec(),
                ),
                None => (None, None),
            };

            result.push(BranchInfo {
                name: bookmark_name.as_str().to_owned(),
                is_local: true,
                remote_name: None,
                target: single_target(ws, local_target),
                is_tracking: remote_refs.iter().any(|&(remote_name, remote_ref)| {
                    remote_name != REMOTE_NAME_FOR_LOCAL_GIT_REPO && remote_ref.is_tracked()
                }),
                is_synced,
                is_absent: false,
                has_conflict: !local_target.is_resolved(),
                ahead,
                behind,
            });
        }

        for &(remote_name, remote_ref) in &remote_refs {
            if remote_name == REMOTE_NAME_FOR_LOCAL_GIT_REPO {
                continue;
            }
            result.push(BranchInfo {
                name: bookmark_name.as_str().to_owned(),
                is_local: false,
                remote_name: Some(remote_name.as_str().to_owned()),
                target: single_target(ws, &remote_ref.target),
                is_tracking: remote_ref.is_tracked(),
                is_synced: remote_ref.target == *local_target,
                is_absent: remote_ref.is_absent(),
                has_conflict: !remote_ref.target.is_resolved(),
                ahead: None,
                behind: None,
            });
        }
    }

    Ok(result)
}

/// How many commits the local branch has that the tracked remote doesn't, and
/// the other way round.
///
/// The two ends of a branch rarely share a history. The range from one end
/// up to the other, without the ancestors that end already has, is what the
/// other end adds. jj counts it the same way, so the numbers here match what
/// jj itself reports.
fn count_ahead_behind(
    ws: &WorkspaceSession,
    local_ids: Vec<CommitId>,
    remote_ids: Vec<CommitId>,
) -> (Option<usize>, Option<usize>) {
    if local_ids.is_empty() || remote_ids.is_empty() {
        return (None, None);
    }

    let repo = ws.repo();
    // the range from the remote up to the local branch, without the ancestors
    // the remote already has: what the local branch adds
    let ahead = revset::walk_revs(repo, &local_ids, &remote_ids)
        .and_then(|revs| revs.count_estimate())
        .ok()
        .map(|(count, _)| count);
    // and the other way round for what the remote has that the local lacks
    let behind = revset::walk_revs(repo, &remote_ids, &local_ids)
        .and_then(|revs| revs.count_estimate())
        .ok()
        .map(|(count, _)| count);

    (ahead, behind)
}

/// The label a conflict term carries, in words someone can act on.
///
/// jj writes the raw commit ids and the role the term plays, like
/// `tqnnuvwv 983d5949 "change a.txt" (rebase destination)`. The role and the
/// description are worth keeping; the ids are noise for someone who only
/// wants to know which side of the conflict to keep.
fn readable_label(label: &str) -> String {
    // the description, in quotes
    let description = label
        .split_once('"')
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(text, _)| text.trim())
        .filter(|text| !text.is_empty());

    // the role the term plays in the rebase
    let role = if label.contains("(rebase destination)") {
        Some("来自新的基线")
    } else if label.contains("(parents of rebased revision)") {
        Some("来自原来的上一个改动")
    } else if label.contains("(rebased revision)") {
        Some("来自这个改动本身")
    } else {
        None
    };

    match (role, description) {
        (Some(role), Some(text)) => format!("{role}（{text}）"),
        (Some(role), None) => role.to_owned(),
        (None, Some(text)) => text.to_owned(),
        (None, None) => label.to_owned(),
    }
}

/// The conflict labels of a merge, rewritten so a reader can tell the sides
/// apart without knowing jj's vocabulary.
fn readable_labels(labels: &ConflictLabels) -> ConflictLabels {
    let rewritten: Vec<String> = labels
        .as_slice()
        .iter()
        .map(|label| readable_label(label))
        .collect();
    ConflictLabels::from_vec(rewritten)
}

/// The commit a ref points at, if it points at exactly one.
fn single_target(ws: &WorkspaceSession, target: &RefTarget) -> Option<crate::messages::CommitId> {
    let mut adds = target.present_adds();
    let commit_id = adds.next()?;
    if adds.next().is_some() {
        // the ref points at more than one commit, so there's no single target
        return None;
    }
    Some(ws.format_commit_id(commit_id))
}

async fn format_tree_changes(
    ws: &WorkspaceSession<'_>,
    changes: &mut Vec<RevChange>,
    mut tree_diff: TreeDiffStream<'_>,
    conflict_labels: Diff<&ConflictLabels>,
) -> Result<()> {
    let store = ws.repo().store();

    while let Some(TreeDiffEntry { path, values }) = tree_diff.next().await {
        let diff = values?;
        let before = &diff.before;
        let after = &diff.after;

        let kind = if before.is_present() && after.is_present() {
            ChangeKind::Modified
        } else if before.is_absent() {
            ChangeKind::Added
        } else {
            ChangeKind::Deleted
        };

        let has_conflict = !after.is_resolved();

        let before_labels = readable_labels(conflict_labels.before);
        let after_labels = readable_labels(conflict_labels.after);
        let before_future =
            conflicts::materialize_tree_value(store, &path, before.clone(), &before_labels);
        let after_future =
            conflicts::materialize_tree_value(store, &path, after.clone(), &after_labels);
        let (before_value, after_value) = try_join!(before_future, after_future)?;

        let hunks = get_value_hunks(3, &path, before_value, after_value).await?;

        changes.push(RevChange {
            path: ws.format_path(path)?,
            kind,
            has_conflict,
            hunks,
        });
    }
    Ok(())
}

async fn get_value_hunks(
    num_context_lines: usize,
    path: &RepoPath,
    left_value: MaterializedTreeValue,
    right_value: MaterializedTreeValue,
) -> Result<Vec<ChangeHunk>> {
    if left_value.is_absent() {
        let right_part = get_value_contents(path, right_value).await?;
        get_unified_hunks(num_context_lines, &[], &right_part)
    } else if right_value.is_present() {
        let left_part = get_value_contents(path, left_value).await?;
        let right_part = get_value_contents(path, right_value).await?;
        get_unified_hunks(num_context_lines, &left_part, &right_part)
    } else {
        let left_part = get_value_contents(path, left_value).await?;
        get_unified_hunks(num_context_lines, &left_part, &[])
    }
}

async fn get_value_contents(path: &RepoPath, value: MaterializedTreeValue) -> Result<Vec<u8>> {
    match value {
        MaterializedTreeValue::Absent => Err(anyhow!(
            "Absent path {path:?} in diff should have been handled by caller"
        )),
        MaterializedTreeValue::File(MaterializedFileValue { mut reader, .. }) => {
            let mut contents = vec![];
            reader.read_to_end(&mut contents).await?;

            let start = &contents[..8000.min(contents.len())]; // same heuristic git uses
            let is_binary = start.contains(&b'\0');
            if is_binary {
                contents.clear();
                contents.push_str("(binary)");
            }
            Ok(contents)
        }
        MaterializedTreeValue::Symlink { target, .. } => Ok(target.into_bytes()),
        MaterializedTreeValue::GitSubmodule(_) => Ok("(submodule)".to_owned().into_bytes()),
        MaterializedTreeValue::FileConflict(file) => {
            let mut hunk_content = vec![];
            conflicts::materialize_merge_result(
                &file.contents,
                &file.labels,
                &mut hunk_content,
                &ConflictMaterializeOptions {
                    marker_style: ConflictMarkerStyle::Git,
                    marker_len: None,
                    merge: MergeOptions {
                        hunk_level: FileMergeHunkLevel::Line,
                        same_change: SameChange::Accept,
                    },
                },
            )?;
            Ok(hunk_content)
        }
        MaterializedTreeValue::OtherConflict { id, labels } => {
            Ok(id.describe(&labels).into_bytes())
        }
        MaterializedTreeValue::Tree(_) => Err(anyhow!("Unexpected tree in diff at path {path:?}")),
        MaterializedTreeValue::AccessDenied(error) => Err(anyhow!(error)),
    }
}

fn get_unified_hunks(
    num_context_lines: usize,
    left_content: &[u8],
    right_content: &[u8],
) -> Result<Vec<ChangeHunk>> {
    let mut hunks = Vec::new();

    for hunk in unified_diff_hunks(
        left_content,
        right_content,
        &UnifiedDiffOptions {
            context: num_context_lines,
            line_diff: LineDiffOptions {
                compare_mode: LineCompareMode::Exact,
            },
        },
    ) {
        let location = ChangeLocation {
            from_file: ChangeRange {
                start: hunk.left_line_range.start,
                len: hunk.left_line_range.len(),
            },
            to_file: ChangeRange {
                start: hunk.right_line_range.start,
                len: hunk.right_line_range.len(),
            },
        };

        let mut lines = Vec::new();
        for (line_type, tokens) in hunk.lines {
            let mut formatter: Vec<u8> = vec![];
            match line_type {
                DiffLineType::Context => {
                    write!(formatter, " ")?;
                }
                DiffLineType::Removed => {
                    write!(formatter, "-")?;
                }
                DiffLineType::Added => {
                    write!(formatter, "+")?;
                }
            }

            for (token_type, content) in tokens {
                match token_type {
                    DiffTokenType::Matching => formatter.write_all(content)?,
                    DiffTokenType::Different => formatter.write_all(content)?, // XXX mark this for GUI display
                }
            }

            lines.push(String::from_utf8_lossy(&formatter).into_owned());
        }

        hunks.push(ChangeHunk {
            location,
            lines: MultilineString { lines },
        });
    }

    Ok(hunks)
}

/**************************/
/* from jj_cli::diff_util */
/**************************/

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnifiedDiffOptions {
    /// Number of context lines to show.
    pub context: usize,
    /// How lines are tokenized and compared.
    pub line_diff: LineDiffOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DiffLineType {
    Context,
    Removed,
    Added,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DiffTokenType {
    Matching,
    Different,
}

type DiffTokenVec<'content> = Vec<(DiffTokenType, &'content [u8])>;

struct UnifiedDiffHunk<'content> {
    left_line_range: Range<usize>,
    right_line_range: Range<usize>,
    lines: Vec<(DiffLineType, DiffTokenVec<'content>)>,
}

impl<'content> UnifiedDiffHunk<'content> {
    fn extend_context_lines(&mut self, lines: impl IntoIterator<Item = &'content [u8]>) {
        let old_len = self.lines.len();
        self.lines.extend(lines.into_iter().map(|line| {
            let tokens = vec![(DiffTokenType::Matching, line)];
            (DiffLineType::Context, tokens)
        }));
        self.left_line_range.end += self.lines.len() - old_len;
        self.right_line_range.end += self.lines.len() - old_len;
    }

    fn extend_removed_lines(&mut self, lines: impl IntoIterator<Item = DiffTokenVec<'content>>) {
        let old_len = self.lines.len();
        self.lines
            .extend(lines.into_iter().map(|line| (DiffLineType::Removed, line)));
        self.left_line_range.end += self.lines.len() - old_len;
    }

    fn extend_added_lines(&mut self, lines: impl IntoIterator<Item = DiffTokenVec<'content>>) {
        let old_len = self.lines.len();
        self.lines
            .extend(lines.into_iter().map(|line| (DiffLineType::Added, line)));
        self.right_line_range.end += self.lines.len() - old_len;
    }
}

fn unified_diff_hunks<'content>(
    left_content: &'content [u8],
    right_content: &'content [u8],
    options: &UnifiedDiffOptions,
) -> Vec<UnifiedDiffHunk<'content>> {
    let mut hunks = vec![];
    let mut current_hunk = UnifiedDiffHunk {
        left_line_range: 1..1,
        right_line_range: 1..1,
        lines: vec![],
    };
    let diff = diff_by_line([left_content, right_content], &options.line_diff);
    let mut diff_hunks = diff.hunks().peekable();
    while let Some(hunk) = diff_hunks.next() {
        match hunk.kind {
            DiffHunkKind::Matching => {
                // Just use the right (i.e. new) content. We could count the
                // number of skipped lines separately, but the number of the
                // context lines should match the displayed content.
                let [_, right] = hunk.contents[..].try_into().unwrap();
                let mut lines = right.split_inclusive(|b| *b == b'\n').fuse();
                if !current_hunk.lines.is_empty() {
                    // The previous hunk line should be either removed/added.
                    current_hunk.extend_context_lines(lines.by_ref().take(options.context));
                }
                let before_lines = if diff_hunks.peek().is_some() {
                    lines.by_ref().rev().take(options.context).collect()
                } else {
                    vec![] // No more hunks
                };
                let num_skip_lines = lines.count();
                if num_skip_lines > 0 {
                    let left_start = current_hunk.left_line_range.end + num_skip_lines;
                    let right_start = current_hunk.right_line_range.end + num_skip_lines;
                    if !current_hunk.lines.is_empty() {
                        hunks.push(current_hunk);
                    }
                    current_hunk = UnifiedDiffHunk {
                        left_line_range: left_start..left_start,
                        right_line_range: right_start..right_start,
                        lines: vec![],
                    };
                }
                // The next hunk should be of DiffHunk::Different type if any.
                current_hunk.extend_context_lines(before_lines.into_iter().rev());
            }
            DiffHunkKind::Different => {
                let (left_lines, right_lines) =
                    unzip_diff_hunks_to_lines(ContentDiff::by_word(hunk.contents).hunks());
                current_hunk.extend_removed_lines(left_lines);
                current_hunk.extend_added_lines(right_lines);
            }
        }
    }
    if !current_hunk.lines.is_empty() {
        hunks.push(current_hunk);
    }
    hunks
}

/// Splits `(left, right)` hunk pairs into `(left_lines, right_lines)`.
#[allow(dead_code)]
fn unzip_diff_hunks_to_lines<'content, I>(
    diff_hunks: I,
) -> (Vec<DiffTokenVec<'content>>, Vec<DiffTokenVec<'content>>)
where
    I: IntoIterator,
    I::Item: Borrow<DiffHunk<'content>>,
{
    let mut left_lines: Vec<DiffTokenVec<'content>> = vec![];
    let mut right_lines: Vec<DiffTokenVec<'content>> = vec![];
    let mut left_tokens: DiffTokenVec<'content> = vec![];
    let mut right_tokens: DiffTokenVec<'content> = vec![];

    for hunk in diff_hunks {
        let hunk = hunk.borrow();
        match hunk.kind {
            DiffHunkKind::Matching => {
                // TODO: add support for unmatched contexts
                debug_assert!(hunk.contents.iter().all_equal());
                for token in hunk.contents[0].split_inclusive(|b| *b == b'\n') {
                    left_tokens.push((DiffTokenType::Matching, token));
                    right_tokens.push((DiffTokenType::Matching, token));
                    if token.ends_with(b"\n") {
                        left_lines.push(mem::take(&mut left_tokens));
                        right_lines.push(mem::take(&mut right_tokens));
                    }
                }
            }
            DiffHunkKind::Different => {
                let [left, right] = hunk.contents[..]
                    .try_into()
                    .expect("hunk should have exactly two inputs");
                for token in left.split_inclusive(|b| *b == b'\n') {
                    left_tokens.push((DiffTokenType::Different, token));
                    if token.ends_with(b"\n") {
                        left_lines.push(mem::take(&mut left_tokens));
                    }
                }
                for token in right.split_inclusive(|b| *b == b'\n') {
                    right_tokens.push((DiffTokenType::Different, token));
                    if token.ends_with(b"\n") {
                        right_lines.push(mem::take(&mut right_tokens));
                    }
                }
            }
        }
    }

    if !left_tokens.is_empty() {
        left_lines.push(left_tokens);
    }
    if !right_tokens.is_empty() {
        right_lines.push(right_tokens);
    }
    (left_lines, right_lines)
}

fn diff_by_line<'input, T: AsRef<[u8]> + ?Sized + 'input>(
    inputs: impl IntoIterator<Item = &'input T>,
    options: &LineDiffOptions,
) -> jj_lib::diff::ContentDiff<'input> {
    // TODO: If we add --ignore-blank-lines, its tokenizer will have to attach
    // blank lines to the preceding range. Maybe it can also be implemented as a
    // post-process (similar to refine_changed_regions()) that expands unchanged
    // regions across blank lines.
    match options.compare_mode {
        LineCompareMode::Exact => {
            ContentDiff::for_tokenizer(inputs, find_line_ranges, CompareBytesExactly)
        }
        LineCompareMode::IgnoreAllSpace => {
            ContentDiff::for_tokenizer(inputs, find_line_ranges, CompareBytesIgnoreAllWhitespace)
        }
        LineCompareMode::IgnoreSpaceChange => {
            ContentDiff::for_tokenizer(inputs, find_line_ranges, CompareBytesIgnoreWhitespaceAmount)
        }
    }
}

fn commit_id_identity(commit_id: &CommitId) -> &CommitId {
    commit_id
}

#[cfg(test)]
mod tests {
    use super::readable_label;

    #[test]
    fn conflict_labels_are_rewritten_into_words() {
        assert_eq!(
            readable_label("tqnnuvwv 983d5949 \"change a.txt\" (rebase destination)"),
            "来自新的基线（change a.txt）"
        );
        assert_eq!(
            readable_label("nxxylmpu fa32b17f \"change both\" (parents of rebased revision)"),
            "来自原来的上一个改动（change both）"
        );
        assert_eq!(
            readable_label("ywknyuol f86298e8 \"create c.txt\" (rebased revision)"),
            "来自这个改动本身（create c.txt）"
        );
    }

    #[test]
    fn a_label_without_a_description_still_names_the_side() {
        assert_eq!(
            readable_label("tqnnuvwv 983d5949 (rebase destination)"),
            "来自新的基线"
        );
    }

    #[test]
    fn an_unrecognised_label_keeps_its_description() {
        assert_eq!(
            readable_label("yszxzttp 43a946c1 \"chain branch A\""),
            "chain branch A"
        );
    }

    #[test]
    fn a_label_with_neither_role_nor_description_is_left_alone() {
        let label = "some other label";
        assert_eq!(readable_label(label), label);
    }
}
