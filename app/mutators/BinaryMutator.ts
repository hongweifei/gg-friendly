import { mutate } from "../ipc";
import { sameChange } from "../ids";
import type { Operand } from "../messages/Operand";
import type { MoveChanges } from "../messages/MoveChanges";
import type { MoveHunk } from "../messages/MoveHunk";
import type { MoveRef } from "../messages/MoveRef";
import type { InsertRevisions } from "../messages/InsertRevisions";
import type { MoveRevisions } from "../messages/MoveRevisions";
import type { AdoptRevision } from "../messages/AdoptRevision";
import type { ChangeId } from "../messages/ChangeId";
import type { CommitId } from "../messages/CommitId";
import RevisionMutator from "./RevisionMutator";
import RefMutator from "./RefMutator";
import type { StoreRef } from "../messages/StoreRef";
import type { CopyHunk } from "../messages/CopyHunk";
import type { CopyChanges } from "../messages/CopyChanges";
export type RichHint = (string | ChangeId | CommitId | Extract<StoreRef, { type: "LocalBookmark" } | { type: "RemoteBookmark" }>)[];
export type Eligibility = { type: "yes", hint: RichHint } | { type: "maybe", hint: string } | { type: "no" };

export default class BinaryMutator {
    #from: Operand;
    #to: Operand | null;
    #ignoreImmutable: boolean;

    constructor(from: Operand, to: Operand | null, ignoreImmutable: boolean) {
        this.#from = from;
        this.#to = to;
        this.#ignoreImmutable = ignoreImmutable;
    }

    canDrag(): Eligibility {
        // can't change finalised commits without a gesture
        if (this.#from.type == "Revision" && this.#from.header.is_immutable) {
            if (this.#ignoreImmutable) {
                return { type: "yes", hint: ["重新基于（这个改动已定稿）", this.#from.header.id.change] };
            } else {
                return { type: "maybe", hint: "（已定稿 —— 打开底部的盾牌开关可以解除保护）" };
            }
        }
        if ((this.#from.type == "Revisions" || this.#from.type == "Change") && this.#from.headers.some((h) => h.is_immutable)) {
            if (this.#ignoreImmutable) {
                return this.#from.type == "Change"
                    ? { type: "yes", hint: [`合并（这个改动已定稿）`, this.#from.headers[0].id.change] }
                    : {
                        type: "yes", hint: this.#from.headers.length == 1
                            ? ["重新基于（这个改动已定稿）", this.#from.headers[0].id.change]
                            : ["重新基于（这些改动已定稿）", this.#from.headers[this.#from.headers.length - 1].id.change, "::", this.#from.headers[0].id.change]
                    };
            }
            else {
                return { type: "maybe", hint: "（已定稿 —— 打开底部的盾牌开关可以解除保护）" };
            }
        }

        // removing a parent changes the child
        if (this.#from.type == "Parent" && this.#from.child.is_immutable) {
            if (this.#ignoreImmutable) {
                return { type: "yes", hint: ["移除上一个改动（这个改动已定稿）", this.#from.child.id.change] };
            } else {
                return { type: "maybe", hint: "（已定稿 —— 打开底部的盾牌开关可以解除保护）" };
            }
        } else if (this.#from.type == "Parent" && this.#from.child.parent_ids.length == 1) {
            return { type: "maybe", hint: "（这个改动只有一个上一个改动）" };
        }

        // can change these listed things
        if (this.#from.type == "Revision") {
            return { type: "yes", hint: ["重新基于改动 ", this.#from.header.id.change] };
        } else if (this.#from.type == "Revisions") {
            return {
                type: "yes", hint: this.#from.headers.length == 1 ? ["重新基于改动 ", this.#from.headers[0].id.change] :
                    ["重新基于这些改动 ", this.#from.headers[this.#from.headers.length - 1].id.change, "::", this.#from.headers[0].id.change]
            };
        } else if (this.#from.type == "Parent") {
            return { type: "yes", hint: ["移除上一个改动 ", this.#from.child.id.change] };
        } else if (this.#from.type == "Change") {
            if (this.#from.hunk) {
                return {
                    type: "yes", hint: this.#from.headers.length == 1 ?
                        [`合并局部内容 ${this.#from.hunk.location.from_file.start}:${this.#from.hunk.location.from_file.start + this.#from.hunk.location.from_file.len}@${this.#from.path.relative_path}，来源改动 `, this.#from.headers[0].id.change] :
                        [`合并局部内容 ${this.#from.hunk.location.from_file.start}:${this.#from.hunk.location.from_file.start + this.#from.hunk.location.from_file.len}@${this.#from.path.relative_path}，来源改动 `, this.#from.headers[this.#from.headers.length - 1].id.change, "::", this.#from.headers[0].id.change]
                };
            } else {
                return {
                    type: "yes", hint: this.#from.headers.length == 1 ?
                        [`合并文件内容 ${this.#from.path.relative_path}，来源改动 `, this.#from.headers[0].id.change] :
                        [`合并文件内容 ${this.#from.path.relative_path}，来源改动 `, this.#from.headers[this.#from.headers.length - 1].id.change, "::", this.#from.headers[0].id.change]
                };
            }
        } else if (this.#from.type == "Ref" && this.#from.ref.type != "Tag") {
            return { type: "yes", hint: ["移动分支 ", this.#from.ref] };
        }

        return { type: "no" };
    }

    canDrop(): Eligibility {
        // generic prohibitions - don't drop undroppables, don't drop on yourself
        if (this.#to == null) {
            return { type: "no" };
        } else if (this.canDrag().type != "yes" && !((this.#from.type == "Revision" || this.#from.type == "Revisions") && this.#to.type == "Merge")) {
            return { type: "no" };
        } else if (this.#from == this.#to) {
            return { type: "no" };
        }

        if (this.#from.type == "Revision") {
            if (this.#to.type == "Revision") {
                return { type: "yes", hint: ["把改动 ", this.#from.header.id.change, " 重新基于 ", this.#to.header.id.change] };
            } else if (this.#to.type == "Parent") {
                if (this.#to.child == this.#from.header) {
                    return { type: "no" };
                } else if (this.#to.child.is_immutable && !this.#ignoreImmutable) {
                    return { type: "maybe", hint: "（已定稿 —— 打开底部的盾牌开关可以解除保护）" };
                } else {
                    return { type: "yes", hint: ["把改动 ", this.#from.header.id.change, " 插入到 ", this.#to.child.id.change] };
                }
            } else if (this.#to.type == "Merge") {
                if (sameChange(this.#to.header.id.change, this.#from.header.id.change)) {
                    return { type: "no" };
                } else {
                    return { type: "yes", hint: ["给改动添加上一个改动 ", this.#to.header.id.change] };
                }
            } else if (this.#to.type == "Repository") {
                return { type: "yes", hint: ["放弃改动 ", this.#from.header.id.commit] };
            }
        }

        if (this.#from.type == "Revisions") {
            if (this.#to.type == "Revision") {
                let toHeader = this.#to.header;
                if (this.#from.headers.some(h => sameChange(h.id.change, toHeader.id.change))) {
                    return { type: "no" }; // target within selected range
                } else {
                    return {
                        type: "yes", hint: this.#from.headers.length == 1
                            ? ["把改动 ", this.#from.headers[0].id.change, " 重新基于 ", this.#to.header.id.change]
                            : ["把这些改动 ", this.#from.headers[this.#from.headers.length - 1].id.change, "::", this.#from.headers[0].id.change, " 重新基于 ", this.#to.header.id.change]
                    };
                }
            } else if (this.#to.type == "Parent") {
                // check that neither before nor after are within the selected range
                let beforeHeader = this.#to.child;
                let afterHeader = this.#to.header;
                if (this.#from.headers.some(h => sameChange(h.id.change, beforeHeader.id.change) || sameChange(h.id.change, afterHeader.id.change))) {
                    return { type: "no" }; // target within selected range
                } else {
                    return {
                        type: "yes", hint: this.#from.headers.length == 1
                            ? ["把改动 ", this.#from.headers[0].id.change, " 插入到 ", beforeHeader.id.change]
                            : ["把这些改动 ", this.#from.headers[this.#from.headers.length - 1].id.change, "::", this.#from.headers[0].id.change, " 插入到 ", beforeHeader.id.change]
                    };
                }
            } else if (this.#to.type == "Repository") {
                return {
                    type: "yes", hint: this.#from.headers.length == 1
                        ? ["放弃改动 ", this.#from.headers[0].id.commit]
                        : ["放弃这些改动 ", this.#from.headers[this.#from.headers.length - 1].id.commit, "::", this.#from.headers[0].id.commit]
                };
            } else if (this.#to.type == "Merge") {
                let toHeader = this.#to.header;
                if (this.#from.headers.some(h => sameChange(h.id.change, toHeader.id.change))) {
                    return { type: "no" }; // target within selected range
                } else {
                    return {
                        type: "yes", hint: this.#from.headers.length == 1
                            ? ["添加上一个改动 ", this.#from.headers[0].id.change, " 到改动 ", toHeader.id.change]
                            : ["添加这些上一个改动 ", this.#from.headers[this.#from.headers.length - 1].id.change, "::", this.#from.headers[0].id.change, " 到改动 ", toHeader.id.change]
                    };
                }
            }
        }

        if (this.#from.type == "Parent") {
            if (this.#to.type == "Repository") {
                return { type: "yes", hint: ["移除上一个改动 ", this.#from.child.id.change] };
            }
        }

        if (this.#from.type == "Change") {
            if (this.#to.type == "Revision") {
                let toHeader = this.#to.header;
                if (this.#from.headers.some((header) => sameChange(header.id.change, toHeader.id.change))) {
                    return { type: "no" };
                } else if (toHeader.is_immutable && !this.#ignoreImmutable) {
                    return { type: "maybe", hint: "（已定稿 —— 打开底部的盾牌开关可以解除保护）" };
                } else {
                    return { type: "yes", hint: [`从改动合并内容 ${this.#from.path.relative_path}，目标改动 `, toHeader.id.change] };
                }
            } else if (this.#to.type == "Repository") {
                let fromOldest = this.#from.headers[this.#from.headers.length - 1];
                if (fromOldest.parent_ids.length == 1) {
                    return { type: "yes", hint: [`恢复改动 ${this.#from.path.relative_path}，来源上一个改动 `, fromOldest.parent_ids[0]] };
                } else {
                    return { type: "maybe", hint: "（这个改动有多个上一个改动）" };
                }
            }
        }

        if (this.#from.type == "Ref" && this.#from.ref.type != "Tag") {
            // local -> rev: set
            if (this.#to.type == "Revision" && this.#from.ref.type == "LocalBookmark") {
                if (sameChange(this.#to.header.id.change, this.#from.header.id.change)) {
                    return { type: "no" };
                } else {
                    return { type: "yes", hint: ["移动分支 ", this.#from.ref, " 到 ", this.#to.header.id.change] };
                }
            }

            // remote -> local: track
            else if (this.#to.type == "Ref" && this.#to.ref.type == "LocalBookmark" &&
                this.#from.ref.type == "RemoteBookmark" && this.#from.ref.bookmark_name == this.#to.ref.bookmark_name) {
                if (this.#from.ref.is_tracked) {
                    return { type: "maybe", hint: "（已经在跟踪了）" };
                } else {
                    return { type: "yes", hint: ["开始跟踪远程分支 ", this.#from.ref] };
                }
            }

            // anything -> anywhere: delete
            else if (this.#to.type == "Repository") {
                if (this.#from.ref.type == "LocalBookmark") {
                    return { type: "yes", hint: ["删除分支 ", this.#from.ref] };
                } else {
                    return {
                        type: "yes", hint: ["从列表中移除远程分支 ", this.#from.ref]
                    };
                }
            }
        }

        return { type: "no" };
    }

    doDrop() {
        if (this.#to == null) {
            console.warn("attempted drop without a target");
            return;
        }
        let options = { ignoreImmutable: this.#ignoreImmutable };

        if (this.#from.type == "Revision") {
            if (this.#to.type == "Revision") {
                // rebase rev onto single target
                mutate<MoveRevisions>("move_revisions", {
                    set: { from: this.#from.header.id, to: this.#from.header.id },
                    parent_ids: [this.#to.header.id]
                }, options);
                return;
            } else if (this.#to.type == "Parent") {
                // insert between targets
                mutate<InsertRevisions>("insert_revisions", {
                    set: { from: this.#from.header.id, to: this.#from.header.id },
                    after_id: this.#to.header.id,
                    before_id: this.#to.child.id
                }, options);
                return;
            } else if (this.#to.type == "Merge") {
                // rebase subtree onto additional targets
                let newParents = [...this.#to.header.parent_ids, this.#from.header.id.commit];
                mutate<AdoptRevision>("adopt_revision", { id: this.#to.header.id, parent_ids: newParents }, options);
                return;
            } else if (this.#to.type == "Repository") {
                // abandon source
                new RevisionMutator([this.#from.header], this.#ignoreImmutable).onAbandon();
                return;
            }
        }

        if (this.#from.type == "Revisions") {
            // headers are ordered newest-first, so [0] is newest and [length-1] is oldest
            const newest = this.#from.headers[0];
            const oldest = this.#from.headers[this.#from.headers.length - 1];

            if (this.#to.type == "Revision") {
                // rebase revset onto single target
                mutate<MoveRevisions>("move_revisions", {
                    set: { from: oldest.id, to: newest.id },
                    parent_ids: [this.#to.header.id]
                }, options);
                return;
            } else if (this.#to.type == "Parent") {
                // insert range between targets
                mutate<InsertRevisions>("insert_revisions", {
                    set: { from: oldest.id, to: newest.id },
                    after_id: this.#to.header.id,
                    before_id: this.#to.child.id
                }, options);
                return;
            } else if (this.#to.type == "Repository") {
                new RevisionMutator(this.#from.headers, this.#ignoreImmutable).onAbandon();
                return;
            } else if (this.#to.type == "Merge") {
                // add all selected revisions as additional parents
                let newParents = [
                    ...this.#to.header.parent_ids,
                    ...this.#from.headers.map(h => h.id.commit)
                ];
                mutate<AdoptRevision>("adopt_revision", { id: this.#to.header.id, parent_ids: newParents }, options);
                return;
            }
            return;
        }

        if (this.#from.type == "Parent") {
            if (this.#to.type == "Repository") {
                // rebase subtree onto fewer targets
                let removeCommit = this.#from.header.id.commit;
                let newParents = this.#from.child.parent_ids.filter(id => id.hex != removeCommit.hex);
                mutate<AdoptRevision>("adopt_revision", { id: this.#from.child.id, parent_ids: newParents }, options);
                return;
            }
        }

        if (this.#from.type == "Change") {
            let fromSingleton = this.#from.headers.length == 1 ? this.#from.headers[0] : null;
            let fromSet = {
                from: this.#from.headers[this.#from.headers.length - 1].id,
                to: this.#from.headers[0].id,
            };
            if (this.#to.type == "Revision") {
                // squash path or subpath to target
                if (this.#from.hunk) {
                    if (!fromSingleton) {
                        return;
                    }
                    mutate<MoveHunk>("move_hunk", {
                        from_id: fromSingleton.id,
                        to_id: this.#to.header.id.commit,
                        path: this.#from.path,
                        hunk: this.#from.hunk
                    }, options);
                } else {
                    mutate<MoveChanges>("move_changes", { from: fromSet, to_id: this.#to.header.id.commit, paths: [this.#from.path] }, options);
                }
                return;
            } else if (this.#to.type == "Repository") {
                // restore path or subpath from source parent to source
                if (this.#from.hunk) {
                    if (!fromSingleton) {
                        return;
                    }
                    mutate<CopyHunk>("copy_hunk", {
                        from_id: fromSingleton.parent_ids[0],
                        to_id: fromSingleton.id,
                        path: this.#from.path,
                        hunk: this.#from.hunk
                    }, options);
                } else {
                    let fromOldest = this.#from.headers[this.#from.headers.length - 1];
                    mutate<CopyChanges>("copy_changes", {
                        from_id: fromOldest.parent_ids[0],
                        to_set: fromSet,
                        paths: [this.#from.path]
                    }, options);
                }
                return;
            }
        }

        if (this.#from.type == "Ref") {
            if (this.#to.type == "Revision") {
                // point ref to revision
                mutate<MoveRef>("move_ref", { to_id: this.#to.header.id, ref: this.#from.ref }, options);
                return;
            } else if (this.#to.type == "Ref" && this.#from.ref.type == "RemoteBookmark") {
                // track remote bookmark with existing local
                new RefMutator(this.#from.ref, this.#ignoreImmutable).onTrack();
            } else if (this.#to.type == "Repository") {
                // various kinds of total or partial deletion
                new RefMutator(this.#from.ref, this.#ignoreImmutable).onDelete();
            }
        }

        console.log("error: unknown validated mutation");
    }
}
