<!--
@component
The detail panel for the selected change: what it says, who made it, what it
touches, and the actions that can be taken on it.
-->

<script lang="ts">
    import type { RevsResult } from "./messages/RevsResult";
    import { get } from "svelte/store";
    import { ignoreToggled, changeSelectEvent, descriptionDraft, dragOverWidget } from "./stores";
    import ChangeObject from "./objects/ChangeObject.svelte";
    import HunkObject from "./objects/HunkObject.svelte";
    import RevisionObject from "./objects/RevisionObject.svelte";
    import RevisionMutator from "./mutators/RevisionMutator";
    import ActionWidget from "./controls/ActionWidget.svelte";
    import Icon from "./controls/Icon.svelte";
    import IdSpan from "./controls/IdSpan.svelte";
    import Pane from "./shell/Pane.svelte";
    import ToggleWidget from "./controls/ToggleWidget.svelte";
    import Zone from "./objects/Zone.svelte";
    import { onEvent } from "./ipc";
    import AuthorSpan from "./controls/AuthorSpan.svelte";
    import ListWidget, { type List } from "./controls/ListWidget.svelte";
    import type { RevChange } from "./messages/RevChange";
    import TimestampSpan from "./controls/TimestampSpan.svelte";
    import TimestampRangeSpan from "./controls/TimestampRangeSpan.svelte";
    import { revisionTitle, branchChips, isUnpublished, isPublished, fileChangeLabel, hasDescription } from "./vocabulary";

    export let revs: Extract<RevsResult, { type: "Detail" }>;

    const CONTEXT = 3;

    $: singleton = revs.set.from.commit.hex == revs.set.to.commit.hex;
    $: newest = revs.headers[0];
    $: oldest = revs.headers[revs.headers.length - 1];
    $: newestImmutable = newest.is_immutable && !$ignoreToggled;
    $: oldestImmutable = oldest.is_immutable && !$ignoreToggled;

    $: mutator = new RevisionMutator(revs.headers, $ignoreToggled);

    $: changeKey = revs.headers.map((h) => `${h.id.change.hex}/${h.id.change.offset ?? ""}`).join(",");
    $: incomingDescription = revs.headers[revs.headers.length - 1].description.lines.join("\n");

    let editableDescription = "";
    let baselineDescription = "";
    let lastChangeKey: string | null = null;
    $: {
        if (changeKey !== lastChangeKey) {
            let draft = get(descriptionDraft);
            editableDescription = draft?.changeKey === changeKey ? draft.text : incomingDescription;
        } else if (editableDescription === baselineDescription) {
            editableDescription = incomingDescription;
        }
        lastChangeKey = changeKey;
        baselineDescription = incomingDescription;
    }
    $: descriptionChanged = editableDescription !== baselineDescription;
    $: descriptionDraft.set(descriptionChanged ? { changeKey, text: editableDescription } : null);
    let resetAuthor = false;
    function updateDescription() {
        mutator.onDescribe(editableDescription, resetAuthor);
    }

    $: firstTimestamp = new Date(
        Math.min(...revs.headers.map((h) => new Date(h.author.timestamp).getTime())),
    ).toISOString();
    $: lastTimestamp = new Date(
        Math.max(...revs.headers.map((h) => new Date(h.author.timestamp).getTime())),
    ).toISOString();
    $: authors = [...new Map(revs.headers.map((h) => [h.author.email, h.author])).values()];

    $: diffStat = formatDiffStat(revs.changes, revs.changes.length + revs.conflicts.length);

    let syntheticChanges = revs.changes
        .concat(
            revs.conflicts.map((conflict) => ({
                kind: "None" as const,
                path: conflict.path,
                has_conflict: true,
                hunks: [conflict.hunk],
            })),
        )
        .sort((a, b) => a.path.relative_path.localeCompare(b.path.relative_path));

    let unset = true;
    let selectedChange = $changeSelectEvent;
    for (let change of syntheticChanges) {
        if (selectedChange?.path?.repo_path === change.path.repo_path) {
            unset = false;
        }
    }
    if (unset) {
        changeSelectEvent.set(syntheticChanges[0]);
    }

    let list: List = {
        getSize() {
            return syntheticChanges.length;
        },
        getSelection() {
            let index =
                syntheticChanges.findIndex((row) => row.path.repo_path == $changeSelectEvent?.path.repo_path) ?? -1;
            return { from: index, to: index };
        },
        selectRow(row: number) {
            $changeSelectEvent = syntheticChanges[row];
        },
        extendSelection(row: number) {
            $changeSelectEvent = syntheticChanges[row];
        },
        editRow(row: number) {},
    };

    onEvent<string>("gg://menu/revision", (event) => mutator.handle(event));

    $: branches = branchChips(newest);
    $: unpublished = isUnpublished(newest);
    $: published = isPublished(newest);

    function minLines(change: RevChange): number {
        let max = 0;
        for (let hunk of change.hunks) {
            max = Math.max(hunk.lines.lines.length, max);
        }
        return Math.min(max, CONTEXT * 2 + 1);
    }

    function formatDiffStat(changes: RevChange[], fileCount: number): string {
        let insertions = 0;
        let deletions = 0;
        for (let change of changes) {
            for (let hunk of change.hunks) {
                for (let line of hunk.lines.lines) {
                    if (line.startsWith("+")) {
                        insertions++;
                    } else if (line.startsWith("-")) {
                        deletions++;
                    }
                }
            }
        }

        let parts = [`${fileCount} 个文件有改动`];
        if (insertions > 0) {
            parts.push(`新增 ${insertions} 行`);
        }
        if (deletions > 0) {
            parts.push(`删除 ${deletions} 行`);
        }
        return parts.join("，");
    }

    function lineColour(line: string): string | null {
        if (line.startsWith("+")) {
            return "add";
        } else if (line.startsWith("-")) {
            return "remove";
        } else {
            return null;
        }
    }
</script>

<Pane titlebar>
    <h2 slot="header" class="header">
        <span class="title">
            {#if singleton}
                {hasDescription(newest)
                    ? revisionTitle(newest)
                    : newest.is_working_copy
                      ? "正在编辑的工作副本"
                      : ""}
            {:else}
                选中了 {revs.headers.length} 个改动
            {/if}
        </span>

        <div class="checkout-commands">
            {#if singleton}
                <ActionWidget
                    tip={newest.is_working_copy ? "继续编辑这个改动" : "把这个改动设为正在编辑"}
                    onClick={mutator.onEdit}
                    disabled={newestImmutable || newest.is_working_copy}>
                    <Icon name="edit-2" /> {newest.is_working_copy ? "继续编辑" : "编辑"}
                </ActionWidget>
            {/if}

            <ActionWidget secondary tip="在下面新建一个改动" onClick={mutator.onNewChild}>
                <Icon name="edit" /> 新建子改动
            </ActionWidget>
        </div>
    </h2>

    <div slot="body" class="body">
        {#if !singleton}
            <div class="description-list">
                {#each revs.headers as header, i}
                    {#if i > 0}<hr class="description-divider" />{/if}
                    <div class="description-row">{header.description.lines.join("\n")}</div>
                {/each}
            </div>
        {:else}
            <textarea
                class="description"
                spellcheck="false"
                disabled={newestImmutable}
                bind:value={editableDescription}
                on:dragenter={dragOverWidget}
                on:dragover={dragOverWidget}
                on:keydown={(ev) => {
                    if (descriptionChanged && ev.key === "Enter" && (ev.metaKey || ev.ctrlKey)) {
                        updateDescription();
                    }
                }}></textarea>
        {/if}

        {#if singleton}
            <div class="signature-commands">
                <span class="who"><AuthorSpan author={newest.author} /></span>
                <TimestampSpan timestamp={newest.author.timestamp} />
                <span class="ids">
                    <IdSpan selectable id={newest.id.change} />
                    <IdSpan selectable id={newest.id.commit} />
                </span>
                {#if branches.length > 0}
                    <span class="branches">
                        {#each branches as branch}
                            <span class="branch">{branch.label}</span>
                        {/each}
                    </span>
                {/if}
                {#if unpublished}
                    <span class="state warn">未上传</span>
                {:else if published}
                    <span class="state ok">已上传</span>
                {/if}
                {#if newest.has_conflict}
                    <span class="state danger">有冲突</span>
                {/if}
                {#if newest.is_immutable}
                    <span class="state">已定稿</span>
                {/if}
                {#if newest.is_working_copy}
                    <span class="state ok">正在编辑</span>
                {/if}

                <span class="spacer"></span>

                <ToggleWidget
                    safe
                    secondary
                    tip="同时把作者改回你自己"
                    bind:checked={resetAuthor}
                    disabled={newestImmutable}
                    on="unlock"
                    off="lock" />
                <ActionWidget
                    tip="保存这段说明"
                    onClick={() => mutator.onDescribe(editableDescription, resetAuthor)}
                    disabled={newestImmutable || !descriptionChanged}>
                    <Icon name="file-text" /> 保存说明
                </ActionWidget>
            </div>
        {:else}
            <div class="signature-commands">
                <span>作者：</span>
                <span>
                    {#each authors as author, ix}
                        <!-- prettier-ignore -->
                        <AuthorSpan {author} />{#if ix < authors.length - 1},&nbsp;
                        {/if}
                    {/each}
                </span>
                <TimestampRangeSpan from={firstTimestamp} to={lastTimestamp} />
            </div>
        {/if}

        {#if revs.parents.length > 0}            <section class="section">
                <h3 class="section-h">上一个改动</h3>
                <Zone operand={{ type: "Merge", header: oldest }} let:target>
                    <div class="parents" class:target>
                        {#each revs.parents as parent}
                            <div class="parent">
                                <RevisionObject header={parent} child={oldest} selected={false} noBookmarks />
                            </div>
                        {/each}
                    </div>
                </Zone>
            </section>
        {/if}

        <section class="section files-section">
            <h3 class="section-h">
                改动的文件
                <span class="stat">{diffStat}</span>
            </h3>

            {#if syntheticChanges.length > 0}
                <div class="move-commands">
                    <ActionWidget
                        tip="把这个改动的所有内容并进上一个改动"
                        onClick={mutator.onSquash}
                        disabled={oldestImmutable || oldest.parent_ids.length != 1}>
                        <Icon name="upload" /> 合并到上一个
                    </ActionWidget>

                    {#if singleton}
                        <ActionWidget
                            tip="把上一个改动里被删掉的内容恢复回来"
                            onClick={mutator.onRestore}
                            disabled={newestImmutable || newest.parent_ids.length != 1}>
                            <Icon name="download" /> 从上一个恢复
                        </ActionWidget>
                    {/if}
                </div>

                <div class="files-and-diff">
                    <ListWidget {list} type="Change" descendant={$changeSelectEvent?.path.repo_path}>
                        <div class="changes">
                            {#each syntheticChanges as change}
                                <ChangeObject {change} headers={revs.headers} selected={false} />
                            {/each}
                        </div>
                    </ListWidget>

                    <div class="diff-pane">
                        {#each syntheticChanges as change}
                            {#if $changeSelectEvent?.path?.repo_path === change.path.repo_path}
                                <div class="diff-head">
                                    <span class="kind {change.has_conflict ? "danger" : change.kind.toLowerCase()}">
                                        {change.has_conflict
                                            ? "冲突"
                                            : fileChangeLabel(change.kind)}
                                    </span>
                                    <span class="grow"></span>
                                    <span class="hint">内容变化</span>
                                </div>
                                <div class="change" style="--lines: {minLines(change)}" tabindex="-1">
                                    {#each change.hunks as hunk}
                                        <div class="hunk">
                                            <HunkObject header={singleton ? newest : null} path={change.path} {hunk} />
                                        </div>
                                        <pre class="diff">{#each hunk.lines.lines as line}<span class={lineColour(line)}
                                                    ><span class="prefix">{line.slice(0, 1)}</span>{line.slice(1)}</span
                                                >{/each}</pre>
                                    {/each}
                                </div>
                            {/if}
                        {/each}
                    </div>
                </div>
            {:else}
                <p class="no-changes">这个改动还没有改动任何文件。</p>
            {/if}
        </section>
    </div>
</Pane>

<style>
    .header {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        text-wrap: nowrap;
        font-weight: normal;
    }

    .title {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 14.5px;
    }

    .checkout-commands {
        height: 30px;
        padding: 0 3px;
        display: flex;
        align-items: center;
        justify-content: end;
        gap: 6px;
    }

    .body {
        height: 100%;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        margin: 0 -6px -3px -6px;
        padding: 0 6px 3px 6px;
        gap: 0;
    }

    /* the description is a single line of context, the files and the diff
       are what the lower half is really for */
    .description {
        flex: none;
    }

    .description {
        resize: vertical;
        min-height: 52px;
        max-height: 120px;
        overflow: auto;
        font-family: var(--stack-text);
        font-size: 14px;
        line-height: 1.6;
    }

    .description-list {
        min-height: 52px;
        max-height: 120px;
        overflow: auto;
        pointer-events: auto;

        border: 1px solid transparent;
        border-radius: var(--radius-control);
        padding: 1px;

        white-space: pre-wrap;
        user-select: text;

        color: var(--ctp-subtext1);
    }

    .description-row {
        white-space: pre-wrap;
    }

    .description-divider {
        border: none;
        border-top: 1px dashed var(--ctp-overlay0);
        margin: 4px 1px;
    }

    .signature-commands {
        min-height: 30px;
        width: 100%;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 0 3px;
        flex-shrink: 0;
        font-size: 13px;
        flex-wrap: wrap;
    }

    .signature-commands .who {
        color: var(--ctp-text);
        white-space: nowrap;
    }

    .signature-commands .ids {
        display: flex;
        gap: 8px;
        align-items: baseline;
    }

    .signature-commands .branches {
        display: flex;
        gap: 4px;
    }

    .signature-commands .branch {
        font-family: var(--stack-code);
        font-size: 11px;
        padding: 3px 7px;
        border-radius: var(--radius-small);
        background: rgba(32, 159, 181, 0.16);
        color: var(--tag-branch);
    }

    .signature-commands .state {
        font-size: 11.5px;
        padding: 3px 8px;
        border-radius: var(--radius-small);
        background: var(--ctp-surface0);
        color: var(--ctp-subtext1);
    }

    .signature-commands .state.warn {
        background: rgba(254, 100, 11, 0.16);
        color: var(--tag-warn);
    }

    .signature-commands .state.ok {
        background: rgba(64, 160, 43, 0.16);
        color: var(--tag-ok);
    }

    .signature-commands .state.danger {
        background: rgba(210, 15, 57, 0.16);
        color: var(--tag-danger);
    }

    .spacer {
        flex: 1;
    }

    .section {
        border-top: 1px solid var(--ctp-overlay0);
        padding: 12px 3px 0 3px;
        flex-shrink: 0;
    }

    .files-section {
        display: flex;
        flex-direction: column;
        min-height: 0;
        flex: 1;
    }

    .section-h {
        font-size: 12px;
        font-weight: 600;
        letter-spacing: 0.4px;
        color: var(--ctp-subtext1);
        margin-bottom: 9px;
        display: flex;
        align-items: baseline;
        gap: 8px;
    }

    .stat {
        font-weight: 400;
        font-size: 11.5px;
        color: var(--ctp-subtext1);
    }

    /* the file list on the left, the diff on the right */
    .files-and-diff {
        display: grid;
        grid-template-columns: minmax(140px, max-content) minmax(0, 1fr);
        min-height: 0;
        flex: 1;
        border-top: 1px solid var(--ctp-overlay0);
    }

    .files-and-diff > :global(.listbox) {
        border-right: 1px solid var(--ctp-overlay0);
        background: var(--ctp-crust);
    }

    .diff-pane {
        overflow: auto;
        min-width: 0;
        background: var(--ctp-base);
    }

    .diff-head {
        position: sticky;
        top: 0;
        z-index: 1;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 6px 12px;
        background: var(--ctp-crust);
        border-bottom: 1px solid var(--ctp-overlay0);
    }

    .diff-head .kind {
        font-size: 11px;
        line-height: 1;
        padding: 4px 8px;
        border-radius: var(--radius-small);
        background: var(--ctp-surface0);
        color: var(--ctp-text);
        flex: none;
    }

    .diff-head .kind.added {
        background: rgba(64, 160, 43, 0.16);
        color: var(--tag-ok);
    }

    .diff-head .kind.deleted {
        background: rgba(210, 15, 57, 0.16);
        color: var(--tag-danger);
    }

    .diff-head .kind.danger {
        background: rgba(210, 15, 57, 0.16);
        color: var(--tag-danger);
    }

    .diff-head .path {
        font-family: var(--stack-code);
        font-size: 12.5px;
        color: var(--ctp-text);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .diff-head .grow {
        flex: 1;
    }

    .diff-head .hint {
        font-size: 11.5px;
        color: var(--ctp-subtext1);
        white-space: nowrap;
    }

    .parents {
        padding: 0 3px;
        max-height: 34px;
        overflow: auto;
    }

    .parent {
        padding: 2px 0;
    }

    .move-commands {
        display: flex;
        gap: 6px;
        margin-bottom: 8px;
    }

    .no-changes {
        font-size: 13.5px;
        color: var(--ctp-subtext1);
        margin: 0 0 8px;
    }

    .changes {
        display: flex;
        flex-direction: column;
        pointer-events: auto;
        overflow-x: hidden;
        overflow-y: auto;
        scrollbar-color: var(--ctp-text) var(--ctp-crust);
        flex: 1;
        min-height: 0;
        padding-bottom: 20px;
    }

    .file-row {
        display: flex;
        align-items: baseline;
        gap: 8px;
        padding: 7px 3px 3px 3px;
        font-size: 13px;
    }

    .change {
        font-size: small;
        margin: 0;
        pointer-events: auto;
        overflow-x: auto;
        overflow-y: scroll;
        scrollbar-color: var(--ctp-text) var(--ctp-base);
        min-height: calc(var(--lines) * 1em);
    }

    .change::-webkit-scrollbar {
        width: 6px;
        height: 6px;
    }

    .change::-webkit-scrollbar-thumb {
        background-color: var(--ctp-text);
        border-radius: 6px;
    }

    .change::-webkit-scrollbar-track {
        background-color: var(--ctp-base);
    }

    .hunk {
        margin: 0;
        text-align: center;
        background: var(--ctp-mantle);
    }

    .diff {
        margin: 0;
        background: var(--ctp-base);
        user-select: text;
    }

    /* the +/-/space is a diff marker, not content - excluding it from selection keeps it out of copies */
    .diff .prefix {
        user-select: none;
    }

    .add {
        color: var(--diff-add);
    }

    .remove {
        color: var(--diff-remove);
    }

    .target {
        color: black;
        background: var(--ctp-flamingo);
    }
</style>
