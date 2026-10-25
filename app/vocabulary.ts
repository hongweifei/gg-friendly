import type { RevHeader } from "./messages/RevHeader";
import type { StoreRef } from "./messages/StoreRef";
import type { ChangeKind } from "./messages/ChangeKind";
import type { ChangeId } from "./messages/ChangeId";
import type { CommitId } from "./messages/CommitId";
import type { BranchInfo } from "./messages/BranchInfo";

/**
 * A change's prefix is only as long as it needs to be unique, so the rest of
 * the id follows it. Eight characters is what the detail pane shows.
 */
export function shortId(id: ChangeId | CommitId): string {
    return id.prefix + id.rest.substring(0, 8 - id.prefix.length);
}

/**
 * Jujutsu's vocabulary is unfamiliar to most people, so the interface says
 * things in plain words and keeps the jj term alongside where the user is
 * likely to need to look it up.
 */

export const terms = {
    revision: "改动",
    revisions: "改动",
    revisionSet: "一组改动",
    workingCopy: "工作副本",
    parent: "上一个改动",
    parents: "上一个改动",
    bookmark: "分支",
    bookmarks: "分支",
    remoteBookmark: "远程分支",
    tag: "标签",
    change: "改动内容",
    changes: "改动内容",
    conflict: "冲突",
    conflicts: "冲突",
    immutable: "已定稿",
    empty: "空改动",
    file: "文件",
    files: "文件",
} as const;

/** a revision's title, or a placeholder when it has none */
export function revisionTitle(header: RevHeader): string {
    let first = header.description.lines[0]?.trim();
    return first ? first : "(还没有描述)";
}

export function hasDescription(header: RevHeader): boolean {
    return header.description.lines[0]?.trim() !== "";
}

/** the short identifier shown for a revision */
export function revisionId(header: RevHeader): string {
    return header.id.change.prefix;
}

/**
 * Bookmarks shown on a revision. A local bookmark and the remote bookmarks it
 * tracks are the same branch, so only one chip is drawn for them.
 */
export function branchChips(header: RevHeader): { label: string; synced: boolean; tracked: boolean }[] {
    let result: { label: string; synced: boolean; tracked: boolean }[] = [];

    for (let ref of header.refs) {
        if (ref.type == "LocalBookmark") {
            result.push({
                label: ref.bookmark_name,
                synced: ref.is_synced,
                tracked: ref.tracking_remotes.length > 0,
            });
        }
    }

    for (let ref of header.refs) {
        if (ref.type == "RemoteBookmark" && !ref.is_synced) {
            result.push({
                label: `${ref.bookmark_name}@${ref.remote_name}`,
                synced: false,
                tracked: ref.is_tracked,
            });
        }
    }

    return result;
}

/**
 * Whether a branch has reached the remote. A local branch that has a
 * matching remote branch has been uploaded; one without hasn't.
 *
 * This matches the "已上传" and "未上传" filters, which ask jj the same
 * question, so a card's label and the filter that finds it always agree.
 */
export function isPublished(header: RevHeader): boolean {
    let hasLocal = header.refs.some((ref: StoreRef) => ref.type == "LocalBookmark");
    let hasRemote = header.refs.some((ref: StoreRef) => ref.type == "RemoteBookmark");
    return hasLocal && hasRemote;
}

export function isUnpublished(header: RevHeader): boolean {
    let hasLocal = header.refs.some((ref: StoreRef) => ref.type == "LocalBookmark");
    let hasRemote = header.refs.some((ref: StoreRef) => ref.type == "RemoteBookmark");
    return hasLocal && !hasRemote;
}

export function fileChangeLabel(kind: ChangeKind): string {
    switch (kind) {
        case "Added":
            return "新增";
        case "Deleted":
            return "删除";
        case "Modified":
            return "修改";
        default:
            return "未改动";
    }
}

/**
 * The action shown when a revision is picked up for editing. jj calls this
 * "checkout", but the user is really just choosing which change to work on.
 */
export function editActionLabel(header: RevHeader): string {
    return header.is_working_copy ? "继续编辑" : "编辑这个改动";
}

/**
 * A revset that picks out the changes sitting on the given local branches.
 *
 * jj matches branch names with glob patterns by default, so a branch called
 * `feature*` would also match `feature-x`. The names are matched exactly
 * instead, which is what the branch list shows.
 */
export function branchesRevset(names: string[]): string {
    if (names.length == 0) {
        return "none()";
    }

    return names
        .map((name) => `bookmarks(exact:"${name.replace(/["\\]/g, "\\$&")}")`)
        .join(" | ");
}
